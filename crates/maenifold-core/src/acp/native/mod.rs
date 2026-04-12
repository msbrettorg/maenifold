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
use process::{configure_new_process_group, log_io_task_exit, reap_child, spawn_stderr_reader};
use sacp::{Client, ConnectionTo, schema as sacps};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use types::{InitInfo, reply_agent_died};

const REQUEST_QUEUE_CAPACITY: usize = 64;

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
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

#[allow(clippy::too_many_arguments)]
async fn run_agent(
    command: String,
    cwd: PathBuf,
    env: HashMap<String, String>,
    client: HeadlessClient,
    init: Arc<StdMutex<Option<InitInfo>>>,
    shared_conn: Arc<StdMutex<Option<ConnectionTo<sacp::Agent>>>>,
    alive: Arc<AtomicBool>,
    mut request_rx: mpsc::Receiver<AgentCommand>,
    mut control_rx: mpsc::UnboundedReceiver<AgentCommand>,
    stderr_tx: broadcast::Sender<String>,
) {
    if trace_enabled() {
        eprintln!("[acp] spawning agent: {}", command);
    }
    let mut command_builder = tokio::process::Command::new("sh");
    command_builder
        .args(["-c", &command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(&cwd)
        .envs(&env)
        .kill_on_drop(true);
    configure_new_process_group(&mut command_builder);
    let mut child = match command_builder.spawn() {
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

    let stderr_task = child
        .stderr
        .take()
        .map(|stderr| spawn_stderr_reader(stderr, stderr_tx));
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
            clear_shared_conn(&shared_conn);
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
            clear_shared_conn(&shared_conn);
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
    match init.lock() {
        Ok(mut slot) => *slot = Some(init_info),
        Err(error) => {
            clear_shared_conn(&shared_conn);
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: Failed to store ACP init info: {}", error);
            }
            tracing::error!("Failed to store ACP init info: {error}");
            if let Some(task) = io_task.take() {
                task.abort();
                log_io_task_exit(task.await);
            }
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    }
    match shared_conn.lock() {
        Ok(mut slot) => *slot = Some(conn.clone()),
        Err(error) => {
            alive.store(false, Ordering::SeqCst);
            if trace_enabled() {
                eprintln!("[acp] error: Failed to store ACP connection: {}", error);
            }
            tracing::error!("Failed to store ACP connection: {error}");
            if let Some(task) = io_task.take() {
                task.abort();
                log_io_task_exit(task.await);
            }
            reap_child(&mut child, stderr_task, None).await;
            return;
        }
    }

    let mut request_closed = false;
    let mut control_closed = false;

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
                clear_shared_conn(&shared_conn);
                alive.store(false, Ordering::SeqCst);
                drain_session_commands_after_io_task_death(&mut request_rx, &mut control_rx);
                break;
            }
            cmd = control_rx.recv(), if !control_closed => {
                match cmd {
                    Some(cmd) => Some(cmd),
                    None => {
                        control_closed = true;
                        continue;
                    }
                }
            }
            cmd = request_rx.recv(), if !request_closed => {
                match cmd {
                    Some(cmd) => Some(cmd),
                    None => {
                        request_closed = true;
                        continue;
                    }
                }
            }
            else => break,
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
        }
    }

    clear_shared_conn(&shared_conn);
    alive.store(false, Ordering::SeqCst);
    if let Some(task) = io_task.take() {
        task.abort();
    }
    reap_child(&mut child, stderr_task, None).await;
}

#[cfg(test)]
async fn close_session_with(client: &HeadlessClient, session_id: String) -> Result<()> {
    client.remove_session(&session_id).await;
    Ok(())
}

fn drain_session_commands_after_io_task_death(
    request_rx: &mut mpsc::Receiver<AgentCommand>,
    control_rx: &mut mpsc::UnboundedReceiver<AgentCommand>,
) {
    while let Ok(command) = control_rx.try_recv() {
        let _ = reply_agent_died(command);
    }

    while let Ok(command) = request_rx.try_recv() {
        let _ = reply_agent_died(command);
    }
}

fn clear_shared_conn(shared_conn: &Arc<StdMutex<Option<ConnectionTo<sacp::Agent>>>>) {
    if let Ok(mut slot) = shared_conn.lock() {
        slot.take();
    }
}

pub fn spawn_agent(
    command: String,
    cwd: PathBuf,
    env: HashMap<String, String>,
    auto_grant_permissions: bool,
) -> AgentHandle {
    let (request_tx, request_rx) = mpsc::channel(REQUEST_QUEUE_CAPACITY);
    let (control_tx, control_rx) = mpsc::unbounded_channel();
    let init = Arc::new(StdMutex::new(None));
    let conn = Arc::new(StdMutex::new(None));
    let alive = Arc::new(AtomicBool::new(true));
    let client = HeadlessClient::new(cwd.clone(), auto_grant_permissions);
    let (stderr_tx, _) = broadcast::channel(64);

    tokio::task::spawn_blocking({
        let init = Arc::clone(&init);
        let conn = Arc::clone(&conn);
        let alive = Arc::clone(&alive);
        let client = client.clone();
        let stderr_tx = stderr_tx.clone();
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
                    command, cwd, env, client, init, conn, alive, request_rx, control_rx, stderr_tx,
                )
                .await;
            });
        }
    });

    AgentHandle {
        mailbox: CommandMailbox::new(request_tx, control_tx),
        init,
        conn,
        alive,
        client,
        stderr: stderr_tx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol as acp;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum AgentLoopExit {
        IoTaskDied,
        Shutdown,
        MailboxesClosed,
    }

    fn test_session_opts() -> SessionOpts {
        SessionOpts {
            session_id: None,
            cwd: PathBuf::from("."),
            auto_grant_permissions: true,
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: None,
            mcp_servers: Vec::new(),
        }
    }

    fn assert_agent_died(error: AgentError, expected_session_id: &str) {
        match error {
            AgentError::AgentDied { session_id } => {
                assert_eq!(session_id, expected_session_id);
            }
            other => {
                panic!("expected AgentDied for session '{expected_session_id}', got {other:?}")
            }
        }
    }

    fn maybe_drain_queued_command(command: AgentCommand, exit: AgentLoopExit) -> bool {
        if exit != AgentLoopExit::IoTaskDied {
            return false;
        }

        reply_agent_died(command)
    }

    #[tokio::test]
    async fn queued_prompt_with_context_only_maps_to_agent_died_for_io_task_exit() {
        let (shutdown_reply, shutdown_rx) = oneshot::channel();
        assert!(!maybe_drain_queued_command(
            AgentCommand::PromptWithContext {
                session_id: "session-shutdown".to_string(),
                prompt: "hello".to_string(),
                context_blocks: vec![acp::ContentBlock::Text(acp::TextContent::new(
                    "ctx".to_string(),
                ))],
                reply: shutdown_reply,
            },
            AgentLoopExit::Shutdown,
        ));
        assert!(
            shutdown_rx.await.is_err(),
            "explicit shutdown should not synthesize AgentDied"
        );

        let (closed_reply, closed_rx) = oneshot::channel();
        assert!(!maybe_drain_queued_command(
            AgentCommand::PromptWithContext {
                session_id: "session-closed".to_string(),
                prompt: "hello".to_string(),
                context_blocks: vec![acp::ContentBlock::Text(acp::TextContent::new(
                    "ctx".to_string(),
                ))],
                reply: closed_reply,
            },
            AgentLoopExit::MailboxesClosed,
        ));
        assert!(
            closed_rx.await.is_err(),
            "ordinary mailbox closure should not synthesize AgentDied"
        );

        let (died_reply, died_rx) = oneshot::channel();
        assert!(maybe_drain_queued_command(
            AgentCommand::PromptWithContext {
                session_id: "session-died".to_string(),
                prompt: "hello".to_string(),
                context_blocks: vec![acp::ContentBlock::Text(acp::TextContent::new(
                    "ctx".to_string(),
                ))],
                reply: died_reply,
            },
            AgentLoopExit::IoTaskDied,
        ));
        assert_agent_died(
            died_rx
                .await
                .expect("io_task death reply should be delivered")
                .expect_err("io_task death should map queued prompt_with_context to AgentDied"),
            "session-died",
        );
    }

    #[tokio::test]
    async fn io_task_drain_skips_non_session_scoped_commands() {
        let (new_session_reply, new_session_rx) = oneshot::channel();
        assert!(!maybe_drain_queued_command(
            AgentCommand::NewSession {
                opts: test_session_opts(),
                reply: new_session_reply,
            },
            AgentLoopExit::IoTaskDied,
        ));
        assert!(
            new_session_rx.await.is_err(),
            "new_session should not be rewritten as AgentDied"
        );

        let (list_sessions_reply, list_sessions_rx) = oneshot::channel();
        assert!(!maybe_drain_queued_command(
            AgentCommand::ListSessions {
                cwd: None,
                reply: list_sessions_reply,
            },
            AgentLoopExit::IoTaskDied,
        ));
        assert!(
            list_sessions_rx.await.is_err(),
            "list_sessions should not be rewritten as AgentDied"
        );

        assert!(!maybe_drain_queued_command(
            AgentCommand::Shutdown,
            AgentLoopExit::IoTaskDied,
        ));
    }

    #[tokio::test]
    async fn close_session_cleanup_is_local_only() {
        let client = HeadlessClient::new(PathBuf::from("."), true);
        client.register_session("session-local-only").await;

        close_session_with(&client, "session-local-only".to_string())
            .await
            .expect("close session should succeed as local cleanup");

        assert!(
            !client.has_session("session-local-only").await,
            "local cleanup should remove tracked session state"
        );
    }

    #[tokio::test]
    async fn close_session_cleanup_is_idempotent() {
        let client = HeadlessClient::new(PathBuf::from("."), true);

        close_session_with(&client, "session-idempotent".to_string())
            .await
            .expect("closing a missing session should still succeed");

        assert!(
            !client.has_session("session-idempotent").await,
            "idempotent cleanup should leave no tracked session state behind"
        );
    }
}
