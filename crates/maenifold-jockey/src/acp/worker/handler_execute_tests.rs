use super::*;
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use serde_json::json;
use std::path::PathBuf;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Clone, Default)]
struct CapturingAgent {
    prompts: Arc<Mutex<Vec<acp::PromptRequest>>>,
}

#[tokio::test(flavor = "current_thread")]
async fn handler_execute_advertises_complete_node_schema_in_prompt_meta() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let key = super::super::pool::pool_key("schema-session", "mock-runtime", "mock-role");
            let agent = CapturingAgent::default();
            let prompts = agent.prompts.clone();
            let _health_tx = insert_mock_connection(&key, agent).await;
            let schema = json!({
                "type": "object",
                "required": ["verdict"],
                "properties": {
                    "verdict": {"enum": ["PASS", "FAIL"]}
                }
            });

            let (delta_tx, _delta_rx) = mpsc::channel(16);
            let (result_tx, result_rx) = oneshot::channel();

            handle_execute(
                key.clone(),
                "mock-runtime",
                "unused-binary".to_string(),
                Vec::new(),
                Vec::new(),
                acp::ClientCapabilities::default(),
                "mock-role".to_string(),
                "schema-session".to_string(),
                "produce structured output".to_string(),
                Vec::new(),
                None,
                "/".to_string(),
                delta_tx,
                result_tx,
                true,
                Vec::new(),
                Some(vec!["complete_node".to_string()]),
                Some(schema.clone()),
                Vec::new(),
                None,
                None,
            )
            .await;

            let result = result_rx.await.expect("worker result sent");
            assert!(result.is_ok(), "prompt should complete: {result:?}");

            let captured = prompts.lock().unwrap().clone();
            assert_eq!(captured.len(), 1);
            let meta = captured[0]
                .meta
                .as_ref()
                .expect("complete_node prompt should carry tool catalog metadata");
            let tool = meta
                .get("tools")
                .and_then(|value| value.as_array())
                .and_then(|tools| tools.first())
                .expect("tool catalog should contain complete_node");
            assert_eq!(tool.get("name"), Some(&json!("complete_node")));
            assert_eq!(tool.get("input_schema"), Some(&schema));

            CONN_MAP.with(|map| {
                map.borrow_mut().remove(&key);
            });
        })
        .await;
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
                prompts.lock().unwrap().push(request);
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

#[tokio::test(flavor = "current_thread")]
async fn handler_execute_forwards_multipart_prompt_blocks_to_upstream_agent() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let key = super::super::pool::pool_key("app-session", "mock-runtime", "mock-role");
            let agent = CapturingAgent::default();
            let prompts = agent.prompts.clone();
            let _health_tx = insert_mock_connection(&key, agent).await;

            let image =
                acp::ContentBlock::Image(acp::ImageContent::new("iVBORw0KGgo=", "image/png"));
            let audio = acp::ContentBlock::Audio(acp::AudioContent::new("UklGRg==", "audio/wav"));
            let text = acp::ContentBlock::Text(acp::TextContent::new("keep this text block"));
            let prompt_blocks = vec![image.clone(), audio.clone(), text.clone()];

            let (delta_tx, _delta_rx) = mpsc::channel(16);
            let (result_tx, result_rx) = oneshot::channel();

            handle_execute(
                key.clone(),
                "mock-runtime",
                "unused-binary".to_string(),
                Vec::new(),
                Vec::new(),
                acp::ClientCapabilities::default(),
                "mock-role".to_string(),
                "app-session".to_string(),
                "fallback text should not replace prompt_blocks".to_string(),
                Vec::new(),
                Some(prompt_blocks.clone()),
                "/".to_string(),
                delta_tx,
                result_tx,
                true,
                Vec::new(),
                None,
                None,
                Vec::new(),
                None,
                None,
            )
            .await;

            let result = result_rx.await.expect("worker result sent");
            assert!(result.is_ok(), "prompt should complete: {result:?}");

            let captured = prompts.lock().unwrap().clone();
            assert_eq!(captured.len(), 1);
            assert_eq!(captured[0].prompt, prompt_blocks);

            CONN_MAP.with(|map| {
                map.borrow_mut().remove(&key);
            });
        })
        .await;
}
