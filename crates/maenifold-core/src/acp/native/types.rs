/// ACP native client types — errors, options, responses, commands, and handle.
use agent_client_protocol as acp;
use agent_client_protocol_schema as acp_schema;
use sacp::{ConnectionTo, schema as sacps};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{Duration, Instant};

use super::client::HeadlessClient;

pub type Result<T> = std::result::Result<T, AgentError>;
pub type AcpxError = AgentError;
type Reply<T> = oneshot::Sender<Result<T>>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("ACP error: {0}")]
    Acp(String),
    #[error("Command queue full")]
    CommandQueueFull,
    #[error("{message}")]
    Timeout { message: String },
    #[error("Agent process failed to start: {0}")]
    Spawn(String),
    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Agent died for session '{session_id}'")]
    AgentDied { session_id: String },
    #[error("Channel closed")]
    ChannelClosed,
}

impl From<acp::Error> for AgentError {
    fn from(error: acp::Error) -> Self {
        Self::Acp(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOpts {
    pub session_id: Option<String>,
    pub cwd: PathBuf,
    pub auto_grant_permissions: bool,
    pub acp_connection_timeout_seconds: Option<u64>,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub thought_level: Option<String>,
    pub config_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub mcp_servers: Vec<acp::McpServer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionInfo {
    pub session_id: String,
    pub modes: Vec<acp::SessionMode>,
    pub models: Vec<acp::ModelInfo>,
    pub config_options: Vec<acp::SessionConfigOption>,
}

#[derive(Debug, Clone)]
pub struct AcpxSessionInfo {
    pub session_id: String,
    pub title: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub thought: Option<String>,
    pub stop_reason: Option<String>,
    pub error: Option<String>,
    pub usage: Option<AcpxUsage>,
}

pub type AcpxResponse = AgentResponse;

/// Token usage summary for an ACP prompt turn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcpxUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub thought_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub content: Vec<acp::ToolCallContent>,
    pub locations: Vec<acp::ToolCallLocation>,
}

impl From<acp::ToolCall> for ToolCallRecord {
    fn from(value: acp::ToolCall) -> Self {
        let acp::ToolCall {
            tool_call_id,
            title,
            kind,
            status,
            content,
            locations,
            raw_input,
            raw_output,
            ..
        } = value;

        Self {
            id: Some(tool_call_id.to_string()),
            name: serialized_name(kind),
            title: Some(title),
            status: serialized_name(status),
            arguments: raw_input,
            output: raw_output,
            content,
            locations,
        }
    }
}

fn serialized_name<T: serde::Serialize>(value: T) -> Option<String> {
    match serde_json::to_value(value).ok()? {
        serde_json::Value::String(value) => Some(value),
        _ => None,
    }
}

#[derive(Clone)]
pub(crate) enum CommandSender {
    Unbounded(mpsc::UnboundedSender<AgentCommand>),
    Bounded(mpsc::Sender<AgentCommand>),
}

impl CommandSender {
    fn try_send(&self, command: AgentCommand) -> Result<()> {
        match self {
            Self::Unbounded(tx) => tx.send(command).map_err(|_| AgentError::ChannelClosed),
            Self::Bounded(tx) => tx.try_send(command).map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => AgentError::CommandQueueFull,
                mpsc::error::TrySendError::Closed(_) => AgentError::ChannelClosed,
            }),
        }
    }
}

pub(crate) fn reply_agent_died(command: AgentCommand) -> bool {
    match command {
        AgentCommand::LoadSession {
            session_id, reply, ..
        } => {
            let _ = reply.send(Err(AgentError::AgentDied { session_id }));
            true
        }
        AgentCommand::Prompt {
            session_id, reply, ..
        } => {
            let _ = reply.send(Err(AgentError::AgentDied { session_id }));
            true
        }
        AgentCommand::PromptWithContext {
            session_id, reply, ..
        } => {
            let _ = reply.send(Err(AgentError::AgentDied { session_id }));
            true
        }
        AgentCommand::NewSession { .. }
        | AgentCommand::ListSessions { .. }
        | AgentCommand::Shutdown => false,
    }
}

#[derive(Clone)]
pub(crate) struct CommandMailbox {
    request_tx: CommandSender,
    control_tx: CommandSender,
}

impl CommandMailbox {
    pub(crate) fn new(
        request_tx: mpsc::Sender<AgentCommand>,
        control_tx: mpsc::UnboundedSender<AgentCommand>,
    ) -> Self {
        Self {
            request_tx: CommandSender::Bounded(request_tx),
            control_tx: CommandSender::Unbounded(control_tx),
        }
    }

    fn enqueue_request(&self, command: AgentCommand) -> Result<()> {
        self.request_tx.try_send(command)
    }

    fn enqueue_control(&self, command: AgentCommand) -> Result<()> {
        self.control_tx.try_send(command)
    }

    #[cfg(test)]
    fn bounded_for_test(
        request_tx: mpsc::Sender<AgentCommand>,
        control_tx: mpsc::Sender<AgentCommand>,
    ) -> Self {
        Self {
            request_tx: CommandSender::Bounded(request_tx),
            control_tx: CommandSender::Bounded(control_tx),
        }
    }
}

pub enum AgentCommand {
    NewSession {
        opts: SessionOpts,
        reply: Reply<SessionInfo>,
    },
    ListSessions {
        cwd: Option<PathBuf>,
        reply: Reply<Vec<AcpxSessionInfo>>,
    },
    LoadSession {
        session_id: String,
        opts: SessionOpts,
        reply: Reply<SessionInfo>,
    },
    Prompt {
        session_id: String,
        prompt: String,
        reply: Reply<AgentResponse>,
    },
    PromptWithContext {
        session_id: String,
        prompt: String,
        context_blocks: Vec<acp::ContentBlock>,
        reply: Reply<AgentResponse>,
    },
    Shutdown,
}

#[derive(Clone, Default)]
pub(super) struct InitInfo {
    pub(super) agent_info: Option<acp::Implementation>,
    pub(super) capabilities: acp::AgentCapabilities,
    pub(super) auth_methods: Vec<acp::AuthMethod>,
}

pub struct AgentHandle {
    pub(crate) mailbox: CommandMailbox,
    pub(super) init: Arc<StdMutex<Option<InitInfo>>>,
    pub(super) conn: Arc<StdMutex<Option<ConnectionTo<sacp::Agent>>>>,
    pub(super) alive: Arc<AtomicBool>,
    pub(super) client: HeadlessClient,
    pub(super) stderr: broadcast::Sender<String>,
}

impl AgentHandle {
    fn agent_died(session_id: &str) -> AgentError {
        AgentError::AgentDied {
            session_id: session_id.to_string(),
        }
    }

    fn channel_closed_or_agent_died(&self, session_id: Option<&str>) -> AgentError {
        match session_id {
            Some(session_id) if !self.is_alive() => Self::agent_died(session_id),
            _ => AgentError::ChannelClosed,
        }
    }

    async fn request<T>(&self, command: impl FnOnce(Reply<T>) -> AgentCommand) -> Result<T> {
        let (reply, rx) = oneshot::channel();
        self.mailbox.enqueue_request(command(reply))?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }

    async fn session_request<T>(
        &self,
        session_id: &str,
        command: impl FnOnce(Reply<T>) -> AgentCommand,
    ) -> Result<T> {
        if !self.is_alive() {
            return Err(Self::agent_died(session_id));
        }

        let (reply, rx) = oneshot::channel();
        self.mailbox
            .enqueue_request(command(reply))
            .map_err(|_| self.channel_closed_or_agent_died(Some(session_id)))?;
        rx.await
            .map_err(|_| self.channel_closed_or_agent_died(Some(session_id)))?
    }

    pub fn agent_info(&self) -> Option<acp::Implementation> {
        self.init
            .lock()
            .ok()
            .and_then(|init| init.as_ref().and_then(|init| init.agent_info.clone()))
    }
    pub fn capabilities(&self) -> Option<acp::AgentCapabilities> {
        self.init
            .lock()
            .ok()
            .and_then(|init| init.as_ref().map(|init| init.capabilities.clone()))
    }
    pub fn auth_methods(&self) -> Option<Vec<acp::AuthMethod>> {
        self.init
            .lock()
            .ok()
            .and_then(|init| init.as_ref().map(|init| init.auth_methods.clone()))
    }
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }
    pub(crate) fn connection(&self) -> Result<ConnectionTo<sacp::Agent>> {
        self.conn
            .lock()
            .map_err(|error| AgentError::Acp(format!("ACP connection state poisoned: {error}")))?
            .as_ref()
            .cloned()
            .ok_or_else(|| AgentError::Acp("ACP connection not ready".to_string()))
    }
    pub async fn register_relay_session(&self, session_id: &str, outbound: mpsc::Sender<String>) {
        self.client
            .register_relay_session(session_id.to_string(), outbound)
            .await;
    }
    pub async fn unregister_relay_session(&self, session_id: &str) {
        self.client.unregister_relay_session(session_id).await;
    }
    pub(crate) async fn track_session(&self, session_id: &str) {
        self.client.register_session(session_id.to_string()).await;
    }
    pub(crate) async fn forget_session(&self, session_id: &str) {
        self.client.remove_session(session_id).await;
    }
    pub(crate) async fn has_tracked_sessions(&self) -> bool {
        self.client.has_any_sessions().await
    }
    pub async fn resolve_relay_response(
        &self,
        request_id: u64,
        payload: std::result::Result<serde_json::Value, String>,
    ) -> bool {
        self.client
            .resolve_relay_response(request_id, payload)
            .await
    }
    pub(crate) async fn wait_for_connection(
        &self,
        timeout: Duration,
    ) -> Result<ConnectionTo<sacp::Agent>> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.connection() {
                Ok(conn) => return Ok(conn),
                Err(error) if !self.is_alive() => return Err(error),
                Err(_) if Instant::now() >= deadline => {
                    return Err(AgentError::Timeout {
                        message: format!(
                            "ACP connection not ready after {}ms",
                            timeout.as_millis()
                        ),
                    });
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }
    }
    pub fn subscribe_stderr(&self) -> broadcast::Receiver<String> {
        self.stderr.subscribe()
    }
    pub async fn new_session(&self, opts: SessionOpts) -> Result<SessionInfo> {
        self.request(|reply| AgentCommand::NewSession { opts, reply })
            .await
    }
    pub async fn list_sessions(&self, cwd: Option<PathBuf>) -> Result<Vec<AcpxSessionInfo>> {
        self.request(|reply| AgentCommand::ListSessions { cwd, reply })
            .await
    }
    pub async fn load_session(&self, session_id: &str, opts: SessionOpts) -> Result<SessionInfo> {
        self.session_request(session_id, |reply| AgentCommand::LoadSession {
            session_id: session_id.to_string(),
            opts,
            reply,
        })
        .await
    }
    pub async fn prompt(&self, session_id: &str, prompt: &str) -> Result<AgentResponse> {
        self.session_request(session_id, |reply| AgentCommand::Prompt {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
            reply,
        })
        .await
    }
    pub async fn prompt_with_context(
        &self,
        session_id: &str,
        prompt: &str,
        context_blocks: Vec<acp::ContentBlock>,
    ) -> Result<AgentResponse> {
        self.session_request(session_id, |reply| AgentCommand::PromptWithContext {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
            context_blocks,
            reply,
        })
        .await
    }
    pub async fn cancel(&self, session_id: &str) -> Result<()> {
        if !self.is_alive() {
            return Err(Self::agent_died(session_id));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|error| AgentError::Acp(format!("ACP connection state poisoned: {error}")))?
            .as_ref()
            .cloned()
            .ok_or_else(|| AgentError::Acp("ACP connection not ready".to_string()))?;

        self.client.mark_cancelled(session_id).await;
        conn.send_notification(sacps::CancelNotification::new(session_id.to_string()))
            .map_err(|error| AgentError::Acp(error.to_string()))
    }
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let result = if self.supports_session_close() && self.is_alive() {
            let conn = self.connection()?;
            let request = sacp::UntypedMessage::new(
                "session/close",
                acp_schema::CloseSessionRequest::new(session_id.to_string()),
            )
            .map_err(|error| AgentError::Acp(error.to_string()))?;
            conn.send_request(request)
                .block_task()
                .await
                .map(|_| ())
                .map_err(|error| AgentError::Acp(error.to_string()))
        } else {
            Ok(())
        };

        self.client.remove_session(session_id).await;
        result
    }

    #[cfg(test)]
    pub(crate) async fn register_test_session(&self, session_id: &str) {
        self.client.register_session(session_id).await;
    }

    pub(crate) async fn has_tracked_session(&self, session_id: &str) -> bool {
        self.client.has_session(session_id).await
    }

    pub(crate) fn shutdown_nowait(&self) -> Result<()> {
        self.mailbox.enqueue_control(AgentCommand::Shutdown)
    }

    fn supports_session_close(&self) -> bool {
        self.init
            .lock()
            .ok()
            .and_then(|init| {
                init.as_ref()
                    .and_then(|init| init.capabilities.session_capabilities.close.clone())
            })
            .is_some()
    }
}

#[cfg(test)]
pub(crate) fn test_handle_for_tests(
    capacity: usize,
) -> (
    AgentHandle,
    mpsc::Receiver<AgentCommand>,
    mpsc::Receiver<AgentCommand>,
) {
    let (request_tx, request_rx) = mpsc::channel(capacity);
    let (control_tx, control_rx) = mpsc::channel(capacity);
    (
        AgentHandle {
            mailbox: CommandMailbox::bounded_for_test(request_tx, control_tx),
            init: Arc::new(StdMutex::new(None)),
            conn: Arc::new(StdMutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
            client: HeadlessClient::new(PathBuf::from("."), true),
            stderr: broadcast::channel(16).0,
        },
        request_rx,
        control_rx,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc, time::Duration};

    fn test_session_opts() -> SessionOpts {
        SessionOpts {
            session_id: None,
            cwd: PathBuf::from("."),
            auto_grant_permissions: true,
            acp_connection_timeout_seconds: None,
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

    fn send_agent_died(command: AgentCommand) -> bool {
        reply_agent_died(command)
    }

    fn bounded_mailbox(
        capacity: usize,
    ) -> (
        CommandMailbox,
        mpsc::Receiver<AgentCommand>,
        mpsc::Receiver<AgentCommand>,
    ) {
        let (request_tx, request_rx) = mpsc::channel(capacity);
        let (control_tx, control_rx) = mpsc::channel(capacity);
        (
            CommandMailbox::bounded_for_test(request_tx, control_tx),
            request_rx,
            control_rx,
        )
    }

    fn test_handle(
        capacity: usize,
    ) -> (
        AgentHandle,
        mpsc::Receiver<AgentCommand>,
        mpsc::Receiver<AgentCommand>,
    ) {
        let (mailbox, request_rx, control_rx) = bounded_mailbox(capacity);
        (
            AgentHandle {
                mailbox,
                init: Arc::new(StdMutex::new(None)),
                conn: Arc::new(StdMutex::new(None)),
                alive: Arc::new(AtomicBool::new(true)),
                client: HeadlessClient::new(PathBuf::from("."), true),
                stderr: broadcast::channel(16).0,
            },
            request_rx,
            control_rx,
        )
    }

    fn dead_handle(
        capacity: usize,
    ) -> (
        AgentHandle,
        mpsc::Receiver<AgentCommand>,
        mpsc::Receiver<AgentCommand>,
    ) {
        let (handle, request_rx, control_rx) = test_handle(capacity);
        handle.alive.store(false, Ordering::SeqCst);
        (handle, request_rx, control_rx)
    }

    #[tokio::test]
    async fn request_lane_saturation_fails_fast_with_backpressure() {
        let (handle, _request_rx, _control_rx) = test_handle(1);
        handle
            .mailbox
            .enqueue_request(AgentCommand::Shutdown)
            .expect("seed request queue");

        let result =
            tokio::time::timeout(Duration::from_millis(50), handle.list_sessions(None)).await;

        let response = result.expect("request should fail fast rather than wait");
        assert!(matches!(response, Err(AgentError::CommandQueueFull)));
    }

    #[tokio::test]
    async fn cancel_before_connection_ready_bypasses_both_mailboxes() {
        let (handle, mut request_rx, mut control_rx) = test_handle(1);
        handle
            .mailbox
            .enqueue_request(AgentCommand::Shutdown)
            .expect("seed request queue");

        let cancel = tokio::spawn(async move { handle.cancel("session-1").await });

        let queued = tokio::time::timeout(Duration::from_millis(50), request_rx.recv())
            .await
            .expect("seeded request command should remain readable")
            .expect("seeded request command should exist");
        assert!(
            matches!(queued, AgentCommand::Shutdown),
            "expected to observe only the seeded request command"
        );
        tokio::task::yield_now().await;
        assert!(
            request_rx.try_recv().is_err(),
            "cancel should not enqueue a request-lane command"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "cancel should not enqueue a control-lane command"
        );
        drop(request_rx);
        drop(control_rx);

        let response = tokio::time::timeout(Duration::from_millis(50), cancel)
            .await
            .expect("cancel should finish without any mailbox ack")
            .expect("cancel task should join");
        assert!(
            response.is_err(),
            "cancel should still report an error before the shared connection is ready"
        );
    }

    #[tokio::test]
    async fn shutdown_uses_control_lane_when_request_lane_is_saturated() {
        let (handle, _request_rx, mut control_rx) = test_handle(1);
        handle
            .mailbox
            .enqueue_request(AgentCommand::Shutdown)
            .expect("seed request queue");

        tokio::time::timeout(Duration::from_millis(50), async {
            handle.shutdown_nowait()
        })
        .await
        .expect("shutdown should enqueue without waiting for request capacity")
        .expect("shutdown should use the control lane");

        let command = tokio::time::timeout(Duration::from_millis(50), control_rx.recv()).await;
        assert!(
            matches!(command, Ok(Some(AgentCommand::Shutdown))),
            "shutdown should reach the control lane"
        );
    }

    #[tokio::test]
    async fn cancel_before_connection_ready_returns_error() {
        let (request_tx, mut request_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = mpsc::channel(1);
        drop(control_rx);
        let handle = AgentHandle {
            mailbox: CommandMailbox::bounded_for_test(request_tx, control_tx),
            init: Arc::new(StdMutex::new(None)),
            conn: Arc::new(StdMutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
            client: HeadlessClient::new(PathBuf::from("."), true),
            stderr: broadcast::channel(16).0,
        };

        let result =
            tokio::time::timeout(Duration::from_millis(50), handle.cancel("session-1")).await;

        let response = result.expect("cancel should complete immediately before conn is ready");
        assert!(response.is_err(), "cancel should fail before conn is ready");
        assert!(
            request_rx.try_recv().is_err(),
            "cancel should not touch the request lane while conn is unavailable"
        );
    }

    #[tokio::test]
    async fn cancel_on_dead_handle_returns_agent_died_without_mailbox_traffic() {
        let (handle, mut request_rx, mut control_rx) = dead_handle(1);

        let response = handle
            .cancel("session-dead")
            .await
            .expect_err("dead handle cancel should fail");

        assert_agent_died(response, "session-dead");
        assert!(
            request_rx.try_recv().is_err(),
            "dead-handle cancel should not enqueue a request-lane command"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "dead-handle cancel should not enqueue a control-lane command"
        );
    }

    #[tokio::test]
    async fn shutdown_reports_dead_control_lane_immediately() {
        let (request_tx, mut request_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = mpsc::channel(1);
        drop(control_rx);
        let handle = AgentHandle {
            mailbox: CommandMailbox::bounded_for_test(request_tx, control_tx),
            init: Arc::new(StdMutex::new(None)),
            conn: Arc::new(StdMutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
            client: HeadlessClient::new(PathBuf::from("."), true),
            stderr: broadcast::channel(16).0,
        };

        let result = tokio::time::timeout(Duration::from_millis(50), async {
            handle.shutdown_nowait()
        })
        .await;

        let response =
            result.expect("shutdown should complete immediately when control lane is dead");
        assert!(matches!(response, Err(AgentError::ChannelClosed)));
        assert!(
            request_rx.try_recv().is_err(),
            "shutdown should not fall back to request lane"
        );
    }

    #[tokio::test]
    async fn io_task_death_matrix_maps_queued_request_commands_to_agent_died() {
        let (load_reply, load_rx) = oneshot::channel();
        assert!(send_agent_died(AgentCommand::LoadSession {
            session_id: "session-load".to_string(),
            opts: test_session_opts(),
            reply: load_reply,
        }));
        assert_agent_died(
            load_rx
                .await
                .expect("load reply should be delivered")
                .expect_err("load should fail with AgentDied"),
            "session-load",
        );

        let (prompt_reply, prompt_rx) = oneshot::channel();
        assert!(send_agent_died(AgentCommand::Prompt {
            session_id: "session-prompt".to_string(),
            prompt: "hello".to_string(),
            reply: prompt_reply,
        }));
        assert_agent_died(
            prompt_rx
                .await
                .expect("prompt reply should be delivered")
                .expect_err("prompt should fail with AgentDied"),
            "session-prompt",
        );

        let (prompt_with_context_reply, prompt_with_context_rx) = oneshot::channel();
        assert!(send_agent_died(AgentCommand::PromptWithContext {
            session_id: "session-context".to_string(),
            prompt: "hello".to_string(),
            context_blocks: Vec::new(),
            reply: prompt_with_context_reply,
        }));
        assert_agent_died(
            prompt_with_context_rx
                .await
                .expect("prompt-with-context reply should be delivered")
                .expect_err("prompt-with-context should fail with AgentDied"),
            "session-context",
        );
    }

    #[tokio::test]
    async fn io_task_death_matrix_skips_non_session_scoped_commands() {
        let (new_session_reply, new_session_rx) = oneshot::channel();
        assert!(!send_agent_died(AgentCommand::NewSession {
            opts: test_session_opts(),
            reply: new_session_reply,
        }));
        assert!(
            new_session_rx.await.is_err(),
            "new_session should not be force-mapped to AgentDied"
        );

        let (list_sessions_reply, list_sessions_rx) = oneshot::channel();
        assert!(!send_agent_died(AgentCommand::ListSessions {
            cwd: None,
            reply: list_sessions_reply,
        }));
        assert!(
            list_sessions_rx.await.is_err(),
            "list_sessions should not be force-mapped to AgentDied"
        );

        assert!(!send_agent_died(AgentCommand::Shutdown));
    }

    #[tokio::test]
    async fn queued_and_in_flight_session_scoped_requests_report_agent_died_after_handle_death() {
        let (handle, mut request_rx, _control_rx) = test_handle(2);
        let handle = Arc::new(handle);

        let in_flight = tokio::spawn({
            let handle = Arc::clone(&handle);
            async move { handle.prompt("session-in-flight", "hello").await }
        });

        let in_flight_command = tokio::time::timeout(Duration::from_millis(50), request_rx.recv())
            .await
            .expect("first command should be queued")
            .expect("first command should exist");
        assert!(
            matches!(in_flight_command, AgentCommand::Prompt { .. }),
            "expected in-flight prompt command"
        );

        let queued = tokio::spawn({
            let handle = Arc::clone(&handle);
            async move {
                handle
                    .prompt_with_context("session-queued", "hello", Vec::new())
                    .await
            }
        });

        tokio::task::yield_now().await;
        handle.alive.store(false, Ordering::SeqCst);
        drop(request_rx);
        drop(in_flight_command);

        assert_agent_died(
            in_flight
                .await
                .expect("in-flight task should join")
                .expect_err("in-flight request should fail"),
            "session-in-flight",
        );
        assert_agent_died(
            queued
                .await
                .expect("queued task should join")
                .expect_err("queued request should fail"),
            "session-queued",
        );
    }

    #[tokio::test]
    async fn future_prompt_with_context_on_dead_handle_reports_agent_died() {
        let (handle, request_rx, control_rx) = dead_handle(1);
        drop(request_rx);
        drop(control_rx);

        let response = handle
            .prompt_with_context("session-future", "hello", Vec::new())
            .await
            .expect_err("dead handle request should fail");

        assert_agent_died(response, "session-future");
    }

    #[tokio::test]
    async fn close_session_on_dead_handle_is_ok() {
        let (handle, request_rx, control_rx) = dead_handle(1);
        drop(request_rx);
        drop(control_rx);
        handle.client.register_session("session-close-dead").await;

        handle
            .close_session("session-close-dead")
            .await
            .expect("close_session should be idempotent local cleanup even after handle death");
        assert!(
            !handle.client.has_session("session-close-dead").await,
            "dead-handle close_session should still clear tracked local session state"
        );
    }

    #[tokio::test]
    async fn close_session_does_not_enqueue_request_or_control_command() {
        let (handle, mut request_rx, mut control_rx) = test_handle(1);
        handle.client.register_session("session-close-local").await;

        tokio::time::timeout(
            Duration::from_millis(50),
            handle.close_session("session-close-local"),
        )
        .await
        .expect("close_session should complete without round-tripping through ACP")
        .expect("close_session should succeed as local cleanup");

        assert!(
            request_rx.try_recv().is_err(),
            "close_session should not enqueue a request-lane command"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "close_session should not enqueue a control-lane command"
        );
        assert!(
            !handle.client.has_session("session-close-local").await,
            "close_session should clear tracked local session state without queue traffic"
        );
    }
}
