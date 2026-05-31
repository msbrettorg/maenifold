use super::*;
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use std::path::PathBuf;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Clone, Default)]
struct CapturingAgent {
    prompts: Arc<Mutex<Vec<acp::PromptRequest>>>,
}

async fn insert_mock_connection(
    key: &str,
    agent: CapturingAgent,
) -> tokio::sync::watch::Sender<bool> {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (agent_read, agent_write) = tokio::io::split(agent_stream);

    let delta_slot = Arc::new(Mutex::new(None));
    let ui_client = Arc::new(super::super::super::client::JockeyUiClient {
        delta_slot,
        auto_approve: true,
        workspace_root: PathBuf::from("/"),
    });
    let client_transport = ByteStreams::new(client_write.compat_write(), client_read.compat());
    let (connection_tx, connection_rx) = tokio::sync::oneshot::channel::<ConnectionTo<Agent>>();
    let client_io_task = Client
        .builder()
        .name("jockey-test")
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::RequestPermissionRequest, responder, _connection| {
                    let response = ui_client.request_permission(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let ui_client = ui_client.clone();
                async move |notif: acp::SessionNotification, _connection| {
                    ui_client.session_notification(notif).await
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            client_transport,
            move |connection: ConnectionTo<Agent>| async move {
                connection_tx.send(connection).map_err(|_| {
                    acp::Error::new(
                        acp::ErrorCode::InternalError.into(),
                        "failed to receive ACP connection handle",
                    )
                })?;
                std::future::pending::<acp::Result<()>>().await
            },
        );

    let prompts = agent.prompts.clone();
    let agent_transport = ByteStreams::new(agent_write.compat_write(), agent_read.compat());
    let agent_io_task = Agent
        .builder()
        .on_receive_request(
            async move |request: acp::InitializeRequest, responder, _connection| {
                responder.respond(acp::InitializeResponse::new(request.protocol_version))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: acp::AuthenticateRequest, responder, _connection| {
                responder.respond(acp::AuthenticateResponse::default())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: acp::NewSessionRequest, responder, _connection| {
                responder.respond(acp::NewSessionResponse::new(acp::SessionId::new(
                    "session-1",
                )))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: acp::PromptRequest, responder, _connection| {
                prompts.lock().expect("prompt capture mutex").push(request);
                responder.respond(acp::PromptResponse::new(acp::StopReason::EndTurn))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |_request: acp::CancelNotification, _connection| Ok(()),
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            agent_transport,
            move |_connection: ConnectionTo<Client>| async move {
                std::future::pending::<acp::Result<()>>().await
            },
        );
    tokio::task::spawn_local(agent_io_task);
    tokio::task::spawn_local({
        let client_io_task = client_io_task;
        async move {
            let _ = client_io_task.await;
        }
    });
    let client_conn = connection_rx
        .await
        .expect("receive ACP connection handle for mock client");

    let (health_tx, health_rx) = tokio::sync::watch::channel(true);

    CONN_MAP.with(|map| {
        map.borrow_mut().insert(
            key.to_string(),
            super::super::pool::LiveConnection {
                instance_id: 1,
                conn: std::rc::Rc::new(client_conn),
                session_id: acp::SessionId::new("session-1"),
                resumed: false,
                cwd: "/".to_string(),
                delta_slot: Arc::new(Mutex::new(None)),
                child_pid: None,
                _io_task: tokio::task::spawn_local(async {}),
                health_rx,
            },
        );
    });
    health_tx
}

fn text(value: &str) -> acp::ContentBlock {
    acp::ContentBlock::Text(acp::TextContent::new(value))
}

fn connection_snapshot(key: &str) -> Option<(u64, String, usize)> {
    CONN_MAP.with(|map| {
        map.borrow().get(key).map(|live| {
            (
                live.instance_id,
                live.session_id.to_string(),
                std::rc::Rc::as_ptr(&live.conn) as *const () as usize,
            )
        })
    })
}

async fn execute_prompt(
    key: &str,
    prompt_text: &str,
    role_config_options: Vec<(String, String)>,
) -> Result<AcpExecuteSuccess, AcpExecuteError> {
    let (delta_tx, _delta_rx) = mpsc::channel(16);
    let (result_tx, result_rx) = oneshot::channel();

    handle_execute(
        key.to_string(),
        "mock-runtime",
        "unused-binary".to_string(),
        Vec::new(),
        Vec::new(),
        acp::ClientCapabilities::default(),
        "mock-role".to_string(),
        "app-session".to_string(),
        prompt_text.to_string(),
        Vec::new(),
        Some(vec![text(prompt_text)]),
        "/".to_string(),
        delta_tx,
        result_tx,
        true,
        Vec::new(),
        None,
        None,
        role_config_options,
        None,
        None,
    )
    .await;

    result_rx.await.expect("worker result sent")
}

fn cleanup_key(key: &str) {
    CONN_MAP.with(|map| {
        map.borrow_mut().remove(key);
    });
    super::super::pool::CANCEL_HANDLES.with(|map| {
        map.borrow_mut().remove(key);
    });
    super::super::pool::PROMPT_LOCKS.with(|map| {
        map.borrow_mut().remove(key);
    });
}

#[tokio::test(flavor = "current_thread")]
async fn consecutive_prompts_succeed_on_warm_connection() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let key = super::super::pool::pool_key("app-session", "mock-runtime", "mock-role");
            let agent = CapturingAgent::default();
            let prompts = agent.prompts.clone();
            let _health_tx = insert_mock_connection(&key, agent).await;
            let before = connection_snapshot(&key).expect("preloaded connection exists");

            let first = execute_prompt(&key, "first turn", Vec::new()).await;
            assert!(first.is_ok(), "first prompt should complete: {first:?}");

            let second = execute_prompt(&key, "second turn", Vec::new()).await;
            assert!(second.is_ok(), "second prompt should complete: {second:?}");

            let after = connection_snapshot(&key).expect("connection should remain live");
            let captured = prompts.lock().expect("prompt capture mutex").clone();
            assert_eq!(
                captured.len(),
                2,
                "both prompts should reach the upstream agent"
            );
            assert_eq!(captured[0].prompt, vec![text("first turn")]);
            assert_eq!(captured[1].prompt, vec![text("second turn")]);
            assert_eq!(
                before, after,
                "warm prompt should reuse the existing connection"
            );
            assert_eq!(
                first.expect("first prompt success").session_id,
                second.expect("second prompt success").session_id,
                "warm prompt should stay on the original ACP session",
            );

            cleanup_key(&key);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn consecutive_prompts_survive_config_reapply() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let key = super::super::pool::pool_key("app-session", "mock-runtime", "mock-role");
            let agent = CapturingAgent::default();
            let prompts = agent.prompts.clone();
            let _health_tx = insert_mock_connection(&key, agent).await;

            let first = execute_prompt(&key, "first turn", vec![]).await;
            assert!(
                first.is_ok(),
                "first prompt with empty config options should complete: {first:?}"
            );

            let second = execute_prompt(&key, "second turn", vec![]).await;
            assert!(
                second.is_ok(),
                "second prompt with empty config options should complete: {second:?}"
            );

            let captured = prompts.lock().expect("prompt capture mutex").clone();
            assert_eq!(
                captured.len(),
                2,
                "empty config reapply should remain a no-op"
            );

            cleanup_key(&key);
        })
        .await;
}
