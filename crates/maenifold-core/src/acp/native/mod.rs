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
use session::{cleanup_timed_out_prompt, handle_new_session, handle_prompt, run_with_timeout};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Mutex, mpsc},
    time::Duration,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::InitInfo;

const DEFAULT_SESSION_TIMEOUT_SECONDS: u64 = 300;

async fn reap_child_process(
    child: &mut tokio::process::Child,
    stderr_task: Option<tokio::task::JoinHandle<()>>,
) {
    if let Err(error) = child.start_kill() {
        tracing::debug!("Failed to terminate ACP child before reaping: {error}");
    }
    match child.wait().await {
        Ok(status) => tracing::debug!("ACP child exited with status: {status}"),
        Err(error) => tracing::debug!("Failed to reap ACP child process: {error}"),
    }
    if let Some(stderr_task) = stderr_task
        && let Err(error) = stderr_task.await
    {
        tracing::debug!("ACP stderr reader task failed: {error}");
    }
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

pub fn spawn_agent(command: String, cwd: PathBuf, env: HashMap<String, String>) -> AgentHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let init = Arc::new(StdMutex::new(None));
    let init_task = Arc::clone(&init);
    let alive = Arc::new(AtomicBool::new(true));
    let alive_task = Arc::clone(&alive);
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        tokio::task::LocalSet::new().block_on(&rt, async move {
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
                    alive_task.store(false, Ordering::SeqCst);
                    tracing::error!("Failed to spawn agent: {error}");
                    return;
                }
            };
            let stderr_task = child.stderr.take().map(|stderr| {
                tokio::task::spawn_local(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    loop {
                        match lines.next_line().await {
                            Ok(Some(line)) => tracing::debug!("ACP stderr: {line}"),
                            Ok(None) => break,
                            Err(error) => {
                                tracing::debug!("Failed to read ACP stderr: {error}");
                                break;
                            }
                        }
                    }
                })
            });
            let stdin = child.stdin.take().expect("stdin").compat_write();
            let stdout = child.stdout.take().expect("stdout").compat();
            let sessions = Arc::new(Mutex::new(HashMap::<String, SessionAccumulator>::new()));
            let session_timeouts = Arc::new(Mutex::new(HashMap::<String, u64>::new()));
            let (conn, io_task) = acp::ClientSideConnection::new(
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
            let mut io_task = Some(tokio::task::spawn_local(io_task));
            let init_response = match conn
                .initialize(
                    acp::InitializeRequest::new(acp::ProtocolVersion::V1)
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
                        ),
                )
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    alive_task.store(false, Ordering::SeqCst);
                    tracing::error!("ACP initialize failed: {error}");
                    drop(conn);
                    reap_child_process(&mut child, stderr_task).await;
                    if let Some(io_task) = io_task.take() {
                        log_io_task_exit(io_task.await);
                    }
                    return;
                }
            };
            if init_response.protocol_version != acp::ProtocolVersion::V1 {
                tracing::warn!(
                    "ACP protocol mismatch: requested {}, got {}",
                    acp::ProtocolVersion::V1,
                    init_response.protocol_version
                );
            }
            if let Some(agent_info) = init_response.agent_info.clone() {
                tracing::info!("Connected to {} v{}", agent_info.name, agent_info.version);
            }
            let _ = init_task.lock().map(|mut init| {
                *init = Some(InitInfo {
                    agent_info: init_response.agent_info,
                    capabilities: init_response.agent_capabilities,
                    auth_methods: init_response.auth_methods,
                })
            });
            loop {
                tokio::select! {
                    io_result = async {
                        match io_task.as_mut() {
                            Some(io_task) => Some(io_task.await),
                            None => None,
                        }
                    } => {
                        if let Some(io_result) = io_result {
                            log_io_task_exit(io_result);
                            io_task = None;
                        }
                        break;
                    }
                    cmd = cmd_rx.recv() => {
                        let Some(cmd) = cmd else {
                            break;
                        };
                        match cmd {
                            AgentCommand::Shutdown => break,
                            AgentCommand::NewSession { opts, reply } => {
                                let timeout_seconds = opts.resolved_timeout_seconds();
                                let result = handle_new_session(&conn, opts).await;
                                if let Ok(session_info) = &result {
                                    session_timeouts
                                        .lock()
                                        .await
                                        .insert(session_info.session_id.clone(), timeout_seconds);
                                }
                                let _ = reply.send(result);
                            }
                            AgentCommand::ListSessions { cwd, reply } => {
                                let caps = init_task.lock().ok().and_then(|i| i.as_ref().map(|i| i.capabilities.clone()));
                                let has_list = caps.as_ref().and_then(|c| c.session_capabilities.list.as_ref()).is_some();
                                let result = if !has_list {
                                    Err(AgentError::Acp("agent does not support session/list".to_string()))
                                } else {
                                    conn.list_sessions(acp::ListSessionsRequest::new().cwd(cwd)).await
                                        .map(|resp| resp.sessions.into_iter().map(|s| AcpxSessionInfo { session_id: s.session_id.to_string(), title: s.title, updated_at: s.updated_at }).collect())
                                        .map_err(|error| AgentError::Acp(error.to_string()))
                                };
                                let _ = reply.send(result);
                            }
                            AgentCommand::LoadSession { session_id, opts, reply } => {
                                let caps = init_task.lock().ok().and_then(|i| i.as_ref().map(|i| i.capabilities.clone()));
                                let has_load = caps.as_ref().is_some_and(|c| c.load_session);
                                let timeout_seconds = opts.resolved_timeout_seconds();
                                let result = if !has_load {
                                    Err(AgentError::Acp("agent does not support load_session".to_string()))
                                } else {
                                    let request = acp::LoadSessionRequest::new(acp::SessionId::new(session_id.clone()), opts.cwd)
                                        .mcp_servers(opts.mcp_servers);
                                    conn.load_session(request).await
                                        .map(|resp| SessionInfo {
                                            session_id: session_id.clone(),
                                            modes: resp.modes.as_ref().map(|s| s.available_modes.clone()).unwrap_or_default(),
                                            models: resp.models.as_ref().map(|s| s.available_models.clone()).unwrap_or_default(),
                                            config_options: resp.config_options.unwrap_or_default(),
                                        })
                                        .map_err(|error| AgentError::Acp(error.to_string()))
                                };
                                if result.is_ok() { session_timeouts.lock().await.insert(session_id, timeout_seconds); }
                                let _ = reply.send(result);
                            }
                            AgentCommand::Prompt {
                                session_id,
                                prompt,
                                reply,
                            } => {
                                let timeout_seconds = session_timeouts
                                    .lock()
                                    .await
                                    .get(&session_id)
                                    .copied()
                                    .unwrap_or(DEFAULT_SESSION_TIMEOUT_SECONDS);
                                let timeout_message =
                                    format!("prompt timed out after {timeout_seconds}s");
                                let result = run_with_timeout(
                                    Duration::from_secs(timeout_seconds),
                                    handle_prompt(&conn, &sessions, &session_id, &prompt),
                                    cleanup_timed_out_prompt(&conn, &sessions, &session_id),
                                    timeout_message,
                                )
                                .await;
                                let timed_out = matches!(
                                    result.as_ref(),
                                    Err(AgentError::Acp(message)) if message.contains("timed out")
                                );
                                if timed_out {
                                    session_timeouts.lock().await.remove(&session_id);
                                }
                                let _ = reply.send(result);
                            }
                            AgentCommand::Cancel { session_id, reply } => {
                                let _ = reply.send(
                                    conn.cancel(acp::CancelNotification::new(acp::SessionId::new(
                                        session_id,
                                    )))
                                    .await
                                    .map_err(|error| AgentError::Acp(error.to_string())),
                                );
                            }
                            AgentCommand::CloseSession { session_id } => {
                                session_timeouts.lock().await.remove(&session_id);
                                let _ = conn
                                    .close_session(acp::CloseSessionRequest::new(acp::SessionId::new(
                                        session_id,
                                    )))
                                    .await;
                            }
                        }
                    }
                }
            }
            alive_task.store(false, Ordering::SeqCst);
            drop(conn);
            reap_child_process(&mut child, stderr_task).await;
            if let Some(io_task) = io_task.take() {
                log_io_task_exit(io_task.await);
            }
        });
    });
    AgentHandle {
        cmd_tx,
        init,
        alive,
    }
}
