mod types;
mod accumulator;
mod pool;
mod client;
mod initialize;
mod session_new;
mod session_list;
mod prompt;
mod set_config_option;
mod process;

pub use pool::{AcpxDriver, AgentPool, GLOBAL_AGENT_POOL};
pub use types::*;

use accumulator::SessionAccumulator;
use acp::Agent;
use agent_client_protocol as acp;
use client::HeadlessClient;
use process::{log_io_task_exit, reap_child, spawn_stderr_reader};
use set_config_option::apply_session_config;
use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, mpsc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::InitInfo;

pub(super) fn format_stop_reason(stop_reason: acp::StopReason) -> String {
    serde_json::to_value(stop_reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn usage_to_response(usage: acp::Usage) -> AcpxUsage {
    AcpxUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        thought_tokens: usage.thought_tokens,
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
    let mut stream = conn.subscribe();
    tokio::task::spawn_local(async move {
        while let Ok(msg) = stream.recv().await {
            let dir = match msg.direction {
                acp::StreamMessageDirection::Incoming => "IN",
                acp::StreamMessageDirection::Outgoing => "OUT",
            };
            eprintln!("[ACP-WIRE] {dir}: {:?}", msg.message);
        }
    });
    let mut io_task = Some(tokio::task::spawn_local(io_future));

    let init_info = match initialize::initialize(&conn).await {
        Ok(init_info) => init_info,
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

    if let Some(info) = init_info.agent_info.as_ref() {
        tracing::info!("Connected to {} v{}", info.name, info.version);
    }
    let _ = init.lock().map(|mut slot| {
        *slot = Some(init_info);
    });

    loop {
        let cmd = tokio::select! {
            biased;
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
                let result = session_new::session_new(&conn, opts).await;
                if let Ok(info) = &result {
                    session_timeouts
                        .lock()
                        .await
                        .insert(info.session_id.clone(), timeout_seconds);
                }
                let _ = reply.send(result);
            }
            AgentCommand::ListSessions { cwd, reply } => {
                let _ = reply.send(session_list::list_sessions(&conn, &init, cwd).await);
            }
            AgentCommand::LoadSession {
                session_id,
                opts,
                reply,
            } => {
                let timeout_seconds = opts.resolved_timeout_seconds();
                let result = session_list::load_session(&conn, &init, session_id.clone(), opts).await;
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
                if io_task.is_none() {
                    let _ = reply.send(Err(AgentError::AgentDied {
                        session_id: session_id.clone(),
                    }));
                    continue;
                }
                let result = prompt::prompt_session(
                    &conn,
                    &sessions,
                    &session_timeouts,
                    session_id,
                    prompt,
                )
                .await;
                let _ = reply.send(result);
            }
            AgentCommand::PromptWithContext {
                session_id,
                prompt,
                context_blocks,
                reply,
            } => {
                if io_task.is_none() {
                    let _ = reply.send(Err(AgentError::AgentDied {
                        session_id: session_id.clone(),
                    }));
                    continue;
                }
                let result = prompt::prompt_session_with_blocks(
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
