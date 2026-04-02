mod accumulator;
mod client;
mod pool;
mod types;

pub use pool::{AcpxDriver, AgentPool, GLOBAL_AGENT_POOL};
pub use types::*;

use accumulator::SessionAccumulator;
use acp::Agent;
use agent_client_protocol as acp;
use client::HeadlessClient;
use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Mutex, mpsc},
    task::JoinHandle,
    time::Duration,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::InitInfo;

const DEFAULT_SESSION_TIMEOUT_SECONDS: u64 = 300;

fn spawn_stderr_reader(stderr: tokio::process::ChildStderr) -> JoinHandle<()> {
    tokio::task::spawn_local(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!("ACP stderr: {line}");
        }
    })
}

fn log_io_task_exit<E: std::fmt::Display>(
    result: std::result::Result<std::result::Result<(), E>, tokio::task::JoinError>,
) {
    match result {
        Ok(Ok(())) => tracing::debug!("ACP I/O task exited"),
        Ok(Err(error)) => tracing::warn!("ACP I/O task exited: {error}"),
        Err(error) => tracing::warn!("ACP I/O task join failed: {error}"),
    }
}

async fn initialize_connection(
    conn: &acp::ClientSideConnection,
) -> Result<acp::InitializeResponse> {
    let request = acp::InitializeRequest::new(acp::ProtocolVersion::V1)
        .client_info(acp::Implementation::new(
            "maenifold",
            env!("CARGO_PKG_VERSION"),
        ))
        .client_capabilities(
            acp::ClientCapabilities::new()
                .fs(acp::FileSystemCapabilities::new()
                    .read_text_file(true)
                    .write_text_file(true))
                .terminal(false),
        );

    let response = tokio::time::timeout(Duration::from_secs(30), conn.initialize(request))
        .await
        .map_err(|_| AgentError::Timeout {
            message: "ACP initialize timed out after 30s".to_string(),
        })?
        .map_err(AgentError::from)?;

    if response.protocol_version != acp::ProtocolVersion::V1 {
        return Err(AgentError::Acp(format!(
            "ACP protocol mismatch: requested {}, got {}",
            acp::ProtocolVersion::V1,
            response.protocol_version
        )));
    }

    Ok(response)
}

async fn new_session(conn: &acp::ClientSideConnection, opts: SessionOpts) -> Result<SessionInfo> {
    let timeout_seconds = opts.resolved_timeout_seconds();
    let cwd = opts.cwd.clone();
    let mcp_servers = opts.mcp_servers.clone();
    let response = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.new_session(acp::NewSessionRequest::new(cwd).mcp_servers(mcp_servers)),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("new_session timed out after {timeout_seconds}s"),
    })?
    .map_err(AgentError::from)?;

    let session_id = response.session_id.clone();
    let has_config_options = response.config_options.is_some();
    let config_options = response.config_options.clone().unwrap_or_default();
    let session_info = SessionInfo {
        session_id: session_id.to_string(),
        modes: response
            .modes
            .as_ref()
            .map(|state| state.available_modes.clone())
            .unwrap_or_default(),
        models: response
            .models
            .as_ref()
            .map(|state| state.available_models.clone())
            .unwrap_or_default(),
        config_options: config_options.clone(),
    };

    apply_session_config(conn, session_id, &opts, has_config_options, &config_options).await;
    Ok(session_info)
}

async fn list_sessions(
    conn: &acp::ClientSideConnection,
    init: &StdMutex<Option<InitInfo>>,
    cwd: Option<PathBuf>,
) -> Result<Vec<AcpxSessionInfo>> {
    let has_list = init
        .lock()
        .ok()
        .and_then(|info| info.as_ref().map(|info| info.capabilities.clone()))
        .and_then(|caps| caps.session_capabilities.list)
        .is_some();

    if !has_list {
        return Err(AgentError::Acp(
            "agent does not support session/list".to_string(),
        ));
    }

    conn.list_sessions(acp::ListSessionsRequest::new().cwd(cwd))
        .await
        .map(|response| {
            response
                .sessions
                .into_iter()
                .map(|session| AcpxSessionInfo {
                    session_id: session.session_id.to_string(),
                    title: session.title,
                    updated_at: session.updated_at,
                })
                .collect()
        })
        .map_err(AgentError::from)
}

async fn load_session(
    conn: &acp::ClientSideConnection,
    init: &StdMutex<Option<InitInfo>>,
    session_id: String,
    opts: SessionOpts,
) -> Result<SessionInfo> {
    let has_load = init
        .lock()
        .ok()
        .and_then(|info| info.as_ref().map(|info| info.capabilities.clone()))
        .is_some_and(|caps| caps.load_session);

    if !has_load {
        return Err(AgentError::Acp(
            "agent does not support load_session".to_string(),
        ));
    }

    let timeout_seconds = opts.resolved_timeout_seconds();
    let response = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.load_session(
            acp::LoadSessionRequest::new(acp::SessionId::new(session_id.clone()), opts.cwd.clone())
                .mcp_servers(opts.mcp_servers.clone()),
        ),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("load_session timed out after {timeout_seconds}s"),
    })?
    .map_err(AgentError::from)?;

    let has_config_options = response.config_options.is_some();
    let config_options = response.config_options.unwrap_or_default();

    apply_session_config(
        conn,
        acp::SessionId::new(session_id.clone()),
        &opts,
        has_config_options,
        &config_options,
    )
    .await;

    Ok(SessionInfo {
        session_id,
        modes: response
            .modes
            .as_ref()
            .map(|state| state.available_modes.clone())
            .unwrap_or_default(),
        models: response
            .models
            .as_ref()
            .map(|state| state.available_models.clone())
            .unwrap_or_default(),
        config_options,
    })
}

/// Apply session configuration based on agent capability path.
///
/// **Modern agents** (return `configOptions` from `newSession`): route model,
/// mode, and thought-level through `session/set_config_option` using the
/// config option's `category` field to find the right ID.
///
/// **Legacy agents** (return `modes`/`models`): use `session/set_mode` and
/// the unstable `_session/set_model`.
async fn apply_session_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
    has_config_options: bool,
    config_options: &[acp::SessionConfigOption],
) {
    if has_config_options {
        apply_modern_config(conn, session_id, opts, config_options).await;
    } else {
        apply_legacy_config(conn, session_id.clone(), opts).await;
        apply_extra_config_options(conn, session_id, opts).await;
    }
}

/// Modern path: find config option IDs by category and set via
/// `session/set_config_option`.
async fn apply_modern_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
    config_options: &[acp::SessionConfigOption],
) {
    if let Some(model) = opts.model.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "model") {
            set_config_option(conn, session_id.clone(), config_id, model.to_string()).await;
        }
    }
    if let Some(mode) = opts.mode.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "mode") {
            set_config_option(conn, session_id.clone(), config_id, mode.to_string()).await;
        } else {
            // mode is a standard ACP method — safe to call even for modern agents
            apply_optional(Some(mode.to_string()), "ACP session mode", |mode| {
                conn.set_session_mode(acp::SetSessionModeRequest::new(session_id.clone(), mode))
            })
            .await;
        }
    }
    if let Some(thought_level) = opts.thought_level.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "thought_level") {
            set_config_option(
                conn,
                session_id.clone(),
                config_id,
                thought_level.to_string(),
            )
            .await;
        } else {
            set_config_option(
                conn,
                session_id.clone(),
                "thought_level".to_string(),
                thought_level.to_string(),
            )
            .await;
        }
    }
    apply_extra_config_options(conn, session_id, opts).await;
}

/// Legacy path: use dedicated ACP methods for model and mode.
async fn apply_legacy_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
) {
    apply_optional(opts.model.clone(), "ACP session model", |model| {
        conn.set_session_model(acp::SetSessionModelRequest::new(session_id.clone(), model))
    })
    .await;

    apply_optional(opts.mode.clone(), "ACP session mode", |mode| {
        conn.set_session_mode(acp::SetSessionModeRequest::new(session_id.clone(), mode))
    })
    .await;

    if let Some(thought_level) = opts.thought_level.clone() {
        set_config_option(conn, session_id, "thought_level".to_string(), thought_level).await;
    }
}

/// Config options that apply regardless of modern/legacy path.
async fn apply_extra_config_options(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
) {
    if let Some(config_options) = opts.config_options.clone() {
        for (config_id, value) in config_options {
            if matches!(
                config_id.as_str(),
                "thought_level" | "allowed_tools" | "max_turns"
            ) {
                continue;
            }
            set_config_option(
                conn,
                session_id.clone(),
                config_id,
                config_option_value(value),
            )
            .await;
        }
    }

    if let Some(allowed_tools) = opts.allowed_tools.clone() {
        set_config_option(
            conn,
            session_id.clone(),
            "allowed_tools".to_string(),
            serde_json::to_string(&allowed_tools).unwrap_or_else(|_| "[]".to_string()),
        )
        .await;
    }

    if let Some(max_turns) = opts.max_turns {
        set_config_option(
            conn,
            session_id,
            "max_turns".to_string(),
            max_turns.to_string(),
        )
        .await;
    }
}

/// Find the config option ID for a given semantic category.
fn find_config_id_by_category(
    config_options: &[acp::SessionConfigOption],
    category_name: &str,
) -> Option<String> {
    config_options.iter().find_map(|opt| {
        let matches = match (&opt.category, category_name) {
            (Some(acp::SessionConfigOptionCategory::Model), "model") => true,
            (Some(acp::SessionConfigOptionCategory::Mode), "mode") => true,
            (Some(acp::SessionConfigOptionCategory::ThoughtLevel), "thought_level") => true,
            (Some(acp::SessionConfigOptionCategory::Other(value)), name) => value == name,
            _ => false,
        };
        matches.then(|| opt.id.to_string())
    })
}

fn config_option_value(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value,
        value => value.to_string(),
    }
}

async fn apply_optional<T, F, Fut, R>(value: Option<T>, description: &str, apply: F)
where
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = acp::Result<R>>,
{
    if let Some(value) = value
        && let Err(error) = apply(value).await
    {
        tracing::warn!("Failed to set {description}: {error}");
    }
}

async fn set_config_option(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    config_id: String,
    value: String,
) {
    if let Err(error) = conn
        .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
            session_id,
            config_id.clone(),
            value,
        ))
        .await
    {
        tracing::warn!("Failed to set ACP session config {config_id}: {error}");
    }
}

fn format_stop_reason(stop_reason: acp::StopReason) -> String {
    serde_json::to_value(stop_reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

fn usage_to_response(usage: acp::Usage) -> AcpxUsage {
    AcpxUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        thought_tokens: usage.thought_tokens,
    }
}

async fn prompt_session(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_timeouts: &Arc<Mutex<HashMap<String, u64>>>,
    session_id: String,
    prompt: String,
) -> Result<AgentResponse> {
    prompt_session_with_blocks(conn, sessions, session_timeouts, session_id, prompt, vec![]).await
}

async fn prompt_session_with_blocks(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_timeouts: &Arc<Mutex<HashMap<String, u64>>>,
    session_id: String,
    prompt: String,
    extra_blocks: Vec<acp::ContentBlock>,
) -> Result<AgentResponse> {
    let timeout_seconds = session_timeouts
        .lock()
        .await
        .get(&session_id)
        .copied()
        .unwrap_or(DEFAULT_SESSION_TIMEOUT_SECONDS);

    sessions
        .lock()
        .await
        .insert(session_id.clone(), SessionAccumulator::default());

    let mut content = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))];
    content.extend(extra_blocks);

    let result = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.clone()),
            content,
        )),
    )
    .await;

    let mut response = sessions
        .lock()
        .await
        .remove(&session_id)
        .unwrap_or_default()
        .into_response();

    match result {
        Ok(Ok(prompt_response)) => {
            response.stop_reason = Some(format_stop_reason(prompt_response.stop_reason));
            response.usage = prompt_response.usage.map(usage_to_response);
            Ok(response)
        }
        Ok(Err(error)) => Err(error.into()),
        Err(_) => {
            if let Err(error) = conn
                .cancel(acp::CancelNotification::new(acp::SessionId::new(
                    session_id.clone(),
                )))
                .await
            {
                tracing::warn!(
                    session_id = %session_id,
                    "Failed to cancel timed out ACP prompt: {error}"
                );
            }
            Err(AgentError::Timeout {
                message: format!("prompt timed out after {timeout_seconds}s"),
            })
        }
    }
}

async fn reap_child(
    child: &mut tokio::process::Child,
    stderr_task: Option<JoinHandle<()>>,
    io_task: Option<JoinHandle<acp::Result<()>>>,
) {
    if let Err(error) = child.start_kill() {
        tracing::debug!("Failed to kill ACP child: {error}");
    }
    let _ = child.wait().await;
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    if let Some(task) = io_task {
        log_io_task_exit(task.await);
    }
}

async fn run_agent(
    command: String,
    cwd: PathBuf,
    env: HashMap<String, String>,
    init: Arc<StdMutex<Option<InitInfo>>>,
    alive: Arc<AtomicBool>,
    mut cmd_rx: mpsc::UnboundedReceiver<AgentCommand>,
) {
    let mut child = match tokio::process::Command::new("sh")
        .args(["-c", &command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(&cwd)
        .envs(&env)
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            alive.store(false, Ordering::SeqCst);
            tracing::error!("Failed to spawn agent: {error}");
            return;
        }
    };

    let stderr_task = child.stderr.take().map(spawn_stderr_reader);
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin.compat_write(),
        None => {
            alive.store(false, Ordering::SeqCst);
            tracing::error!("Failed to acquire ACP child stdin");
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout.compat(),
        None => {
            alive.store(false, Ordering::SeqCst);
            tracing::error!("Failed to acquire ACP child stdout");
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    };

    let sessions = Arc::new(Mutex::new(HashMap::<String, SessionAccumulator>::new()));
    let session_timeouts = Arc::new(Mutex::new(HashMap::<String, u64>::new()));
    let (conn, io_future) = acp::ClientSideConnection::new(
        HeadlessClient {
            cwd: cwd.clone(),
            sessions: Arc::clone(&sessions),
        },
        stdin,
        stdout,
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    let conn = Rc::new(conn);
    let mut io_task = Some(tokio::task::spawn_local(io_future));

    let response = match initialize_connection(&conn).await {
        Ok(response) => response,
        Err(error) => {
            alive.store(false, Ordering::SeqCst);
            match &error {
                AgentError::Timeout { message } => tracing::warn!("{message}"),
                other => tracing::error!("{other}"),
            }
            drop(conn);
            reap_child(&mut child, stderr_task, io_task.take()).await;
            return;
        }
    };

    if let Some(info) = response.agent_info.as_ref() {
        tracing::info!("Connected to {} v{}", info.name, info.version);
    }
    let _ = init.lock().map(|mut slot| {
        *slot = Some(InitInfo {
            agent_info: response.agent_info,
            capabilities: response.agent_capabilities,
            auth_methods: response.auth_methods,
        });
    });

    loop {
        let cmd = tokio::select! {
            io_result = async {
                match io_task.as_mut() {
                    Some(task) => task.await,
                    None => std::future::pending().await,
                }
            } => {
                log_io_task_exit(io_result);
                io_task = None;
                break;
            }
            cmd = cmd_rx.recv() => cmd,
        };

        let Some(cmd) = cmd else {
            break;
        };

        match cmd {
            AgentCommand::Shutdown => break,
            AgentCommand::NewSession { opts, reply } => {
                let timeout_seconds = opts.resolved_timeout_seconds();
                let result = new_session(&conn, opts).await;
                if let Ok(info) = &result {
                    session_timeouts
                        .lock()
                        .await
                        .insert(info.session_id.clone(), timeout_seconds);
                }
                let _ = reply.send(result);
            }
            AgentCommand::ListSessions { cwd, reply } => {
                let _ = reply.send(list_sessions(&conn, &init, cwd).await);
            }
            AgentCommand::LoadSession {
                session_id,
                opts,
                reply,
            } => {
                let timeout_seconds = opts.resolved_timeout_seconds();
                let result = load_session(&conn, &init, session_id.clone(), opts).await;
                if result.is_ok() {
                    session_timeouts
                        .lock()
                        .await
                        .insert(session_id, timeout_seconds);
                }
                let _ = reply.send(result);
            }
            AgentCommand::Prompt {
                session_id,
                prompt,
                reply,
            } => {
                let result =
                    prompt_session(&conn, &sessions, &session_timeouts, session_id, prompt).await;
                let _ = reply.send(result);
            }
            AgentCommand::PromptWithContext {
                session_id,
                prompt,
                context_blocks,
                reply,
            } => {
                let result = prompt_session_with_blocks(
                    &conn,
                    &sessions,
                    &session_timeouts,
                    session_id,
                    prompt,
                    context_blocks,
                )
                .await;
                let _ = reply.send(result);
            }
            AgentCommand::Cancel { session_id, reply } => {
                if let Some(accumulator) = sessions.lock().await.get_mut(&session_id) {
                    accumulator.mark_in_progress_cancelled();
                }
                let result = conn
                    .cancel(acp::CancelNotification::new(acp::SessionId::new(
                        session_id,
                    )))
                    .await
                    .map_err(AgentError::from);
                let _ = reply.send(result);
            }
            AgentCommand::CloseSession { session_id } => {
                sessions.lock().await.remove(&session_id);
                session_timeouts.lock().await.remove(&session_id);
                if let Err(error) = conn
                    .close_session(acp::CloseSessionRequest::new(acp::SessionId::new(
                        session_id,
                    )))
                    .await
                {
                    tracing::warn!("Failed to close ACP session: {error}");
                }
            }
        }
    }

    alive.store(false, Ordering::SeqCst);
    drop(conn);
    reap_child(&mut child, stderr_task, io_task.take()).await;
}

pub fn spawn_agent(command: String, cwd: PathBuf, env: HashMap<String, String>) -> AgentHandle {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let init = Arc::new(StdMutex::new(None));
    let alive = Arc::new(AtomicBool::new(true));

    tokio::task::spawn_blocking({
        let init = Arc::clone(&init);
        let alive = Arc::clone(&alive);
        move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    alive.store(false, Ordering::SeqCst);
                    tracing::error!("Failed to build ACP runtime: {error}");
                    return;
                }
            };

            tokio::task::LocalSet::new().block_on(&runtime, async move {
                run_agent(command, cwd, env, init, alive, cmd_rx).await;
            });
        }
    });

    AgentHandle {
        cmd_tx,
        init,
        alive,
    }
}
