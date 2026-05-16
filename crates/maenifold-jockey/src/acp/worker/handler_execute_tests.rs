use super::*;
use std::path::PathBuf;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Clone, Default)]
struct CapturingAgent {
    prompts: Arc<Mutex<Vec<acp::PromptRequest>>>,
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for CapturingAgent {
    async fn initialize(
        &self,
        request: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        Ok(acp::InitializeResponse::new(request.protocol_version))
    }

    async fn authenticate(
        &self,
        _request: acp::AuthenticateRequest,
    ) -> acp::Result<acp::AuthenticateResponse> {
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        _request: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        Ok(acp::NewSessionResponse::new(acp::SessionId::new(
            "session-1",
        )))
    }

    async fn prompt(&self, request: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        self.prompts.lock().unwrap().push(request);
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, _request: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

fn insert_mock_connection(
    key: &str,
    agent: CapturingAgent,
) -> tokio::sync::watch::Sender<bool> {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (agent_read, agent_write) = tokio::io::split(agent_stream);

    let delta_slot = Arc::new(Mutex::new(None));
    let (client_conn, client_io_task) = acp::ClientSideConnection::new(
        super::super::super::client::JockeyUiClient {
            delta_slot,
            auto_approve: true,
            workspace_root: PathBuf::from("/"),
        },
        client_write.compat_write(),
        client_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    let (_agent_conn, agent_io_task) = acp::AgentSideConnection::new(
        agent,
        agent_write.compat_write(),
        agent_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    tokio::task::spawn_local(agent_io_task);

    let (health_tx, health_rx) = tokio::sync::watch::channel(true);
    let child = tokio::process::Command::new("true")
        .spawn()
        .expect("spawn inert child process for LiveConnection test fixture");

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
                available_modes: Vec::new(),
                current_mode: None,
                child_pid: None,
                _child: child,
                _io_task: tokio::task::spawn_local(async move {
                    let _ = client_io_task.await;
                }),
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
            let _health_tx = insert_mock_connection(&key, agent);

            let image =
                acp::ContentBlock::Image(acp::ImageContent::new("iVBORw0KGgo=", "image/png"));
            let audio =
                acp::ContentBlock::Audio(acp::AudioContent::new("UklGRg==", "audio/wav"));
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
