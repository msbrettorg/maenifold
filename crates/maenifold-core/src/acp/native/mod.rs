mod accumulator;
mod client;
mod convert;
mod initialize;
mod pool;
mod process;
mod prompt;
mod session_list;
mod session_new;
mod set_config_option;
mod types;

pub use pool::{AgentPool, GLOBAL_AGENT_POOL};
pub use types::*;

use client::HeadlessClient;
use process::{log_io_task_exit, reap_child, spawn_stderr_reader};
use sacp::{Client, ConnectionTo, schema as sacps};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::InitInfo;

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE").is_ok()
}

pub(super) fn format_stop_reason(stop_reason: sacps::StopReason) -> String {
    serde_json::to_value(stop_reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn usage_to_response(usage: sacps::Usage) -> AcpxUsage {
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
    auto_grant_permissions: bool,
    init: Arc<StdMutex<Option<InitInfo>>>,
    alive: Arc<AtomicBool>,
    mut cmd_rx: mpsc::UnboundedReceiver<AgentCommand>,
) {
    if trace_enabled() {
        eprintln!("[acp] spawning agent: {}", command);
    }
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
            if trace_enabled() {
                eprintln!("[acp] error: {}", error);
            }
            tracing::error!("Failed to spawn agent: {error}");
            return;
        }
    };

    let stderr_task = child.stderr.take().map(spawn_stderr_reader);
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin.compat_write(),
        None => {
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: Failed to acquire ACP child stdin");
            }
            tracing::error!("Failed to acquire ACP child stdin");
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout.compat(),
        None => {
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: Failed to acquire ACP child stdout");
            }
            tracing::error!("Failed to acquire ACP child stdout");
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    };

    let client = HeadlessClient::new(cwd.clone(), auto_grant_permissions);
    let transport = sacp::ByteStreams::new(stdin, stdout);
    let (ready_tx, ready_rx) = oneshot::channel::<
        std::result::Result<(ConnectionTo<sacp::Agent>, InitInfo), AgentError>,
    >();
    let handler_client = client.clone();
    let mut ready_tx = Some(ready_tx);

    let mut io_task = Some(tokio::spawn(async move {
        let permission_client = handler_client.clone();
        let write_client = handler_client.clone();
        let read_client = handler_client.clone();
        let notification_client = handler_client.clone();

        Client
            .builder()
            .on_receive_request(
                async move |request: sacps::RequestPermissionRequest, responder, _cx| {
                    responder
                        .respond_with_result(permission_client.request_permission(request).await)
                },
                sacp::on_receive_request!(),
            )
            .on_receive_request(
                async move |request: sacps::WriteTextFileRequest, responder, _cx| {
                    responder.respond_with_result(write_client.write_text_file(request).await)
                },
                sacp::on_receive_request!(),
            )
            .on_receive_request(
                async move |request: sacps::ReadTextFileRequest, responder, _cx| {
                    responder.respond_with_result(read_client.read_text_file(request).await)
                },
                sacp::on_receive_request!(),
            )
            .on_receive_notification(
                async move |notification: sacps::SessionNotification, _cx| {
                    notification_client.session_notification(notification).await
                },
                sacp::on_receive_notification!(),
            )
            .connect_with(transport, move |conn: ConnectionTo<sacp::Agent>| {
                let mut ready_tx = ready_tx.take();
                async move {
                    let init_result = initialize::initialize(&conn)
                        .await
                        .map(|info| (conn.clone(), info));
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(init_result.clone());
                    }
                    init_result
                        .map(|_| ())
                        .map_err(|error| sacp::util::internal_error(error.to_string()))?;
                    std::future::pending::<std::result::Result<(), sacp::Error>>().await
                }
            })
            .await
    }));

    let (conn, init_info) = match ready_rx.await {
        Ok(Ok((conn, init_info))) => (conn, init_info),
        Ok(Err(error)) => {
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: {}", error);
            }
            tracing::error!("{error}");
            if let Some(task) = io_task.take() {
                let _ = task.await;
            }
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
        Err(_) => {
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: ACP connection closed before initialize completed");
            }
            tracing::error!("ACP connection closed before initialize completed");
            if let Some(task) = io_task.take() {
                log_io_task_exit(task.await);
            }
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    };

    if let Some(info) = init_info.agent_info.as_ref() {
        tracing::info!("Connected to {} v{}", info.name, info.version);
    }
    if trace_enabled() {
        eprintln!("[acp] agent initialized");
    }
    let _ = init.lock().map(|mut slot| *slot = Some(init_info));

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
                let _ = reply.send(session_new::session_new(&conn, &client, opts).await);
            }
            AgentCommand::ListSessions { cwd, reply } => {
                let _ = reply.send(session_list::list_sessions(&conn, &init, cwd).await);
            }
            AgentCommand::LoadSession {
                session_id,
                opts,
                reply,
            } => {
                let _ = reply.send(
                    session_list::load_session(&conn, &client, &init, session_id, opts).await,
                );
            }
            AgentCommand::Prompt {
                session_id,
                prompt,
                reply,
            } => {
                let result = prompt::prompt_session(&conn, &client, session_id, prompt).await;
                let _ = reply.send(result);
            }
            AgentCommand::PromptWithContext {
                session_id,
                prompt,
                context_blocks,
                reply,
            } => {
                let result = prompt::prompt_session_with_blocks(
                    &conn,
                    &client,
                    session_id,
                    prompt,
                    context_blocks,
                )
                .await;
                let _ = reply.send(result);
            }
            AgentCommand::Cancel { session_id, reply } => {
                client.mark_cancelled(&session_id).await;
                let result = conn
                    .send_notification(sacps::CancelNotification::new(session_id))
                    .map_err(|error| AgentError::Acp(error.to_string()));
                let _ = reply.send(result);
            }
            AgentCommand::CloseSession { session_id } => {
                client.remove_session(&session_id).await;
                if let Err(error) = conn
                    .send_request(agent_client_protocol::ClientRequest::CloseSessionRequest(
                        agent_client_protocol::CloseSessionRequest::new(
                            agent_client_protocol::SessionId::new(session_id),
                        ),
                    ))
                    .block_task()
                    .await
                {
                    tracing::warn!("Failed to close ACP session: {error}");
                }
            }
        }
    }

    alive.store(false, Ordering::SeqCst);
    if let Some(task) = io_task.take() {
        task.abort();
    }
    reap_child(&mut child, stderr_task, None).await;
}

pub fn spawn_agent(
    command: String,
    cwd: PathBuf,
    env: HashMap<String, String>,
    auto_grant_permissions: bool,
) -> AgentHandle {
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
                    if trace_enabled() {
                        eprintln!("[acp] error: {}", error);
                    }
                    tracing::error!("Failed to build ACP runtime: {error}");
                    return;
                }
            };

            runtime.block_on(async move {
                run_agent(
                    command,
                    cwd,
                    env,
                    auto_grant_permissions,
                    init,
                    alive,
                    cmd_rx,
                )
                .await;
            });
        }
    });

    AgentHandle {
        cmd_tx,
        init,
        alive,
    }
}
