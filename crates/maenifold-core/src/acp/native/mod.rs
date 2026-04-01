mod accumulator;
mod client;
mod pool;
mod session;
mod types;

pub use pool::{AcpxDriver, AgentPool};
pub use types::*;

use accumulator::SessionAccumulator;
use acp::Agent;
use agent_client_protocol as acp;
use client::HeadlessClient;
use session::{apply_session_config, cleanup_timed_out_prompt, handle_new_session, handle_prompt};
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
    sync::{Mutex, mpsc, oneshot},
    task::JoinHandle,
    time::Duration,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::InitInfo;

const DEFAULT_SESSION_TIMEOUT_SECONDS: u64 = 300;

/// Runs an async operation with a timeout, invoking `on_timeout` and returning
/// [`AgentError::Timeout`] when the deadline is exceeded.
async fn run_with_timeout<T, O, C>(
    duration: Duration,
    operation: O,
    on_timeout: C,
    timeout_message: String,
) -> Result<T>
where
    O: std::future::Future<Output = Result<T>>,
    C: std::future::Future<Output = ()>,
{
    match tokio::time::timeout(duration, operation).await {
        Ok(result) => result,
        Err(_) => {
            on_timeout.await;
            Err(AgentError::Timeout {
                message: timeout_message,
            })
        }
    }
}

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

/// Performs the ACP initialize handshake with a 30-second timeout and protocol
/// version check.
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
                .fs(
                    acp::FileSystemCapabilities::new()
                        .read_text_file(true)
                        .write_text_file(true),
                )
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

/// Dispatches a single [`AgentCommand`], returning `true` when the command loop
/// should exit.
#[allow(clippy::too_many_arguments)]
async fn handle_command(
    cmd: AgentCommand,
    conn: &Rc<acp::ClientSideConnection>,
    init: &StdMutex<Option<InitInfo>>,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_timeouts: &Arc<Mutex<HashMap<String, u64>>>,
    active_prompts: &mut HashMap<String, JoinHandle<()>>,
    prompt_replies: &mut HashMap<String, oneshot::Sender<Result<AgentResponse>>>,
    prompt_result_tx: &mpsc::UnboundedSender<(String, Result<AgentResponse>)>,
) -> bool {
    match cmd {
        AgentCommand::Shutdown => return true,
        AgentCommand::NewSession { opts, reply } => {
            let timeout_seconds = opts.resolved_timeout_seconds();
            let timeout_message = format!("new_session timed out after {timeout_seconds}s");
            let result = run_with_timeout(
                Duration::from_secs(timeout_seconds),
                handle_new_session(conn, opts),
                async {},
                timeout_message,
            )
            .await;
            if let Ok(info) = &result {
                session_timeouts
                    .lock()
                    .await
                    .insert(info.session_id.clone(), timeout_seconds);
            }
            let _ = reply.send(result);
        }
        AgentCommand::ListSessions { cwd, reply } => {
            let caps = init
                .lock()
                .ok()
                .and_then(|i| i.as_ref().map(|i| i.capabilities.clone()));
            let has_list = caps
                .as_ref()
                .and_then(|c| c.session_capabilities.list.as_ref())
                .is_some();
            let result = if !has_list {
                Err(AgentError::Acp(
                    "agent does not support session/list".to_string(),
                ))
            } else {
                conn.list_sessions(acp::ListSessionsRequest::new().cwd(cwd))
                    .await
                    .map(|resp| {
                        resp.sessions
                            .into_iter()
                            .map(|s| AcpxSessionInfo {
                                session_id: s.session_id.to_string(),
                                title: s.title,
                                updated_at: s.updated_at,
                            })
                            .collect()
                    })
                    .map_err(AgentError::from)
            };
            let _ = reply.send(result);
        }
        AgentCommand::LoadSession {
            session_id,
            opts,
            reply,
        } => {
            let has_load = init
                .lock()
                .ok()
                .and_then(|i| i.as_ref().map(|i| i.capabilities.clone()))
                .is_some_and(|c| c.load_session);
            let timeout_seconds = opts.resolved_timeout_seconds();
            let result = if !has_load {
                Err(AgentError::Acp(
                    "agent does not support load_session".to_string(),
                ))
            } else {
                let SessionOpts {
                    cwd, model, mode, thought_level, config_options,
                    allowed_tools, max_turns, timeout_seconds: _, mcp_servers,
                } = opts;
                let timeout_message =
                    format!("load_session timed out after {timeout_seconds}s");
                run_with_timeout(
                    Duration::from_secs(timeout_seconds),
                    async {
                        let resp = conn
                            .load_session(
                                acp::LoadSessionRequest::new(
                                    acp::SessionId::new(session_id.clone()),
                                    cwd,
                                )
                                .mcp_servers(mcp_servers),
                            )
                            .await
                            .map_err(AgentError::from)?;
                        apply_session_config(
                            conn,
                            acp::SessionId::new(session_id.clone()),
                            model, mode, thought_level, config_options,
                            allowed_tools, max_turns,
                        )
                        .await;
                        Ok(SessionInfo {
                            session_id: session_id.clone(),
                            modes: resp.modes.as_ref()
                                .map(|s| s.available_modes.clone())
                                .unwrap_or_default(),
                            models: resp.models.as_ref()
                                .map(|s| s.available_models.clone())
                                .unwrap_or_default(),
                            config_options: resp.config_options.unwrap_or_default(),
                        })
                    },
                    async {},
                    timeout_message,
                )
                .await
            };
            if result.is_ok() {
                session_timeouts.lock().await.insert(session_id, timeout_seconds);
            }
            let _ = reply.send(result);
        }
        AgentCommand::Prompt { session_id, prompt, reply } => {
            if active_prompts.contains_key(&session_id) {
                let _ = reply.send(Err(AgentError::Acp(format!(
                    "prompt already in progress for session '{session_id}'"
                ))));
                return false;
            }
            prompt_replies.insert(session_id.clone(), reply);
            let conn = Rc::clone(conn);
            let sessions = Arc::clone(sessions);
            let session_timeouts = Arc::clone(session_timeouts);
            let prompt_result_tx = prompt_result_tx.clone();
            let task_session_id = session_id.clone();
            let handle = tokio::task::spawn_local(async move {
                let timeout_seconds = session_timeouts
                    .lock().await
                    .get(&task_session_id).copied()
                    .unwrap_or(DEFAULT_SESSION_TIMEOUT_SECONDS);
                let timeout_message = format!("prompt timed out after {timeout_seconds}s");
                let result = run_with_timeout(
                    Duration::from_secs(timeout_seconds),
                    handle_prompt(&conn, &sessions, &task_session_id, &prompt),
                    cleanup_timed_out_prompt(&conn, &sessions, &task_session_id),
                    timeout_message,
                )
                .await;
                if matches!(result.as_ref(), Err(AgentError::Timeout { .. })) {
                    session_timeouts.lock().await.remove(&task_session_id);
                }
                let _ = prompt_result_tx.send((task_session_id, result));
            });
            active_prompts.insert(session_id, handle);
        }
        AgentCommand::Cancel { session_id, reply } => {
            if let Some(handle) = active_prompts.remove(&session_id) {
                handle.abort();
            }
            if let Some(prompt_reply) = prompt_replies.remove(&session_id) {
                let _ = prompt_reply.send(Err(AgentError::Acp(format!(
                    "prompt cancelled for session '{session_id}'"
                ))));
            }
            let result = conn
                .cancel(acp::CancelNotification::new(acp::SessionId::new(
                    session_id.clone(),
                )))
                .await
                .map_err(AgentError::from);
            if let Some(acc) = sessions.lock().await.get_mut(&session_id) {
                acc.mark_in_progress_cancelled();
            }
            let _ = reply.send(result);
        }
        AgentCommand::CloseSession { session_id } => {
            if let Some(handle) = active_prompts.remove(&session_id) {
                handle.abort();
            }
            if let Some(prompt_reply) = prompt_replies.remove(&session_id) {
                let _ = prompt_reply.send(Err(AgentError::Acp(format!(
                    "prompt cancelled because session '{session_id}' was closed"
                ))));
            }
            session_timeouts.lock().await.remove(&session_id);
            if let Err(e) = conn
                .close_session(acp::CloseSessionRequest::new(acp::SessionId::new(
                    session_id,
                )))
                .await
            {
                tracing::warn!("Failed to close ACP session: {e}");
            }
        }
    }
    false
}

async fn reap_child(
    child: &mut tokio::process::Child,
    stderr_task: Option<JoinHandle<()>>,
    io_task: Option<JoinHandle<acp::Result<()>>>,
) {
    if let Err(e) = child.start_kill() {
        tracing::debug!("Failed to kill ACP child: {e}");
    }
    let _ = child.wait().await;
    if let Some(t) = stderr_task {
        let _ = t.await;
    }
    if let Some(t) = io_task {
        log_io_task_exit(t.await);
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
    // compat wrappers required: ACP crate expects futures::io traits.
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
        HeadlessClient { cwd: cwd.clone(), sessions: Arc::clone(&sessions) },
        stdin, stdout,
        |fut| { tokio::task::spawn_local(fut); },
    );
    let conn = Rc::new(conn);
    let mut io_task: Option<JoinHandle<_>> = Some(tokio::task::spawn_local(io_future));
    let response = match initialize_connection(&conn).await {
        Ok(r) => r,
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
    let (prompt_result_tx, mut prompt_result_rx) =
        mpsc::unbounded_channel::<(String, Result<AgentResponse>)>();
    let mut active_prompts = HashMap::<String, JoinHandle<()>>::new();
    let mut prompt_replies = HashMap::<String, oneshot::Sender<Result<AgentResponse>>>::new();
    loop {
        tokio::select! {
            io_result = async {
                match io_task.as_mut() {
                    Some(task) => task.await,
                    None => std::future::pending().await,
                }
            } => {
                log_io_task_exit(io_result);
                io_task = None;
                for (_, h) in active_prompts.drain() { h.abort(); }
                for (sid, reply) in prompt_replies.drain() {
                    let _ = reply.send(Err(AgentError::AgentDied { session_id: sid }));
                }
                break;
            }
            prompt_result = prompt_result_rx.recv() => {
                if let Some((sid, result)) = prompt_result {
                    active_prompts.remove(&sid);
                    if let Some(reply) = prompt_replies.remove(&sid) {
                        let _ = reply.send(result);
                    }
                }
            }
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else {
                    for (_, h) in active_prompts.drain() { h.abort(); }
                    for (sid, reply) in prompt_replies.drain() {
                        let _ = reply.send(Err(AgentError::AgentDied { session_id: sid }));
                    }
                    break;
                };
                if handle_command(
                    cmd, &conn, &init, &sessions, &session_timeouts,
                    &mut active_prompts, &mut prompt_replies, &prompt_result_tx,
                ).await {
                    for (_, h) in active_prompts.drain() { h.abort(); }
                    for (sid, reply) in prompt_replies.drain() {
                        let _ = reply.send(Err(AgentError::AgentDied { session_id: sid }));
                    }
                    break;
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
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(error) => {
                    alive.store(false, Ordering::SeqCst);
                    tracing::error!("Failed to build ACP runtime: {error}");
                    return;
                }
            };
            tokio::task::LocalSet::new().block_on(&rt, async move {
                run_agent(command, cwd, env, init, alive, cmd_rx).await;
            });
        }
    });
    AgentHandle { cmd_tx, init, alive }
}
