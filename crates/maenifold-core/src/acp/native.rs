use agent_client_protocol as acp;
use acp::Agent;
use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::acp_config::AgentsConfig;

pub type Result<T> = std::result::Result<T, AgentError>;
pub type AcpxError = AgentError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("ACP error: {0}")] Acp(String),
    #[error("Agent process failed to start: {0}")] Spawn(String),
    #[error("Agent not found: {0}")] NotFound(String),
    #[error("Channel closed")] ChannelClosed,
}

pub struct SessionOpts {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub thought_level: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<u32>,
}

pub struct AgentResponse { pub text: String, pub stop_reason: Option<String> }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcpxResponse { pub text: String, pub tool_calls: Vec<ToolCallRecord>, pub error: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub id: Option<String>, pub name: Option<String>, pub title: Option<String>,
    pub status: Option<String>, pub arguments: Option<serde_json::Value>,
}

pub struct AgentHandle { pub(crate) cmd_tx: mpsc::UnboundedSender<AgentCommand> }

pub enum AgentCommand {
    NewSession { opts: SessionOpts, reply: oneshot::Sender<Result<String>> },
    Prompt { session_id: String, prompt: String, reply: oneshot::Sender<Result<AgentResponse>> },
    Cancel { session_id: String, reply: oneshot::Sender<Result<()>> },
    CloseSession { session_id: String },
    Shutdown,
}

impl AgentHandle {
    pub async fn new_session(&self, opts: SessionOpts) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(AgentCommand::NewSession { opts, reply: tx }).map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn prompt(&self, session_id: &str, prompt: &str) -> Result<AgentResponse> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(AgentCommand::Prompt {
            session_id: session_id.to_string(), prompt: prompt.to_string(), reply: tx,
        }).map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn cancel(&self, session_id: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(AgentCommand::Cancel { session_id: session_id.to_string(), reply: tx })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn close_session(&self, session_id: &str) {
        let _ = self.cmd_tx.send(AgentCommand::CloseSession { session_id: session_id.to_string() });
    }
}

pub fn spawn_agent(command: String, cwd: PathBuf) -> AgentHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt");
        tokio::task::LocalSet::new().block_on(&rt, async move {
            let mut child = match tokio::process::Command::new("sh")
                .args(["-c", &command]).stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
                .current_dir(&cwd).kill_on_drop(true).spawn() {
                    Ok(child) => child,
                    Err(error) => { tracing::error!("Failed to spawn agent: {error}"); return; }
                };
            let stdin = child.stdin.take().expect("stdin").compat_write();
            let stdout = child.stdout.take().expect("stdout").compat();
            let text = Arc::new(Mutex::new(HashMap::<String, String>::new()));
            let (conn, io_task) = acp::ClientSideConnection::new(
                HeadlessClient { text: Arc::clone(&text) }, stdin, stdout, |fut| { tokio::task::spawn_local(fut); },
            );
            tokio::task::spawn_local(async move {
                if let Err(error) = io_task.await { tracing::warn!("ACP I/O task exited: {error}"); }
            });
            if let Err(error) = conn.initialize(
                acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                    .client_info(acp::Implementation::new("maenifold", env!("CARGO_PKG_VERSION")))
            ).await {
                tracing::error!("ACP initialize failed: {error}");
                return;
            }
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    AgentCommand::Shutdown => break,
                    AgentCommand::NewSession { opts, reply } => { let _ = reply.send(handle_new_session(&conn, opts).await); }
                    AgentCommand::Prompt { session_id, prompt, reply } => {
                        let _ = reply.send(handle_prompt(&conn, &text, &session_id, &prompt).await);
                    }
                    AgentCommand::Cancel { session_id, reply } => {
                        let _ = reply.send(conn.cancel(acp::CancelNotification::new(acp::SessionId::new(session_id)))
                            .await.map_err(|error| AgentError::Acp(error.to_string())));
                    }
                    AgentCommand::CloseSession { session_id } => {
                        let _ = conn.close_session(acp::CloseSessionRequest::new(acp::SessionId::new(session_id))).await;
                    }
                }
            }
        });
    });
    AgentHandle { cmd_tx }
}

async fn handle_new_session(conn: &acp::ClientSideConnection, opts: SessionOpts) -> Result<String> {
    let response = conn.new_session(acp::NewSessionRequest::new(opts.cwd))
        .await.map_err(|error| AgentError::Acp(error.to_string()))?;
    let sid = response.session_id.clone();
    if let Some(model) = opts.model
        && let Err(error) = conn.set_session_model(acp::SetSessionModelRequest::new(sid.clone(), model)).await {
            tracing::warn!("Failed to set ACP session model: {error}");
        }
    set_option(conn, sid.clone(), "thought_level", opts.thought_level).await;
    set_option(conn, sid.clone(), "allowed_tools",
        opts.allowed_tools.map(|tools| serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string()))).await;
    set_option(conn, sid.clone(), "max_turns", opts.max_turns.map(|turns| turns.to_string())).await;
    Ok(sid.to_string())
}

async fn set_option(conn: &acp::ClientSideConnection, session_id: acp::SessionId, config_id: &str, value: Option<String>) {
    if let Some(value) = value
        && let Err(error) = conn.set_session_config_option(acp::SetSessionConfigOptionRequest::new(
            session_id, config_id.to_string(), value
        )).await {
            tracing::warn!("Failed to set ACP session config {config_id}: {error}");
        }
}

async fn handle_prompt(
    conn: &acp::ClientSideConnection,
    text_buf: &Arc<Mutex<HashMap<String, String>>>,
    session_id: &str,
    prompt: &str,
) -> Result<AgentResponse> {
    text_buf.lock().await.insert(session_id.to_string(), String::new());
    let response = conn.prompt(acp::PromptRequest::new(
        acp::SessionId::new(session_id.to_string()),
        vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))],
    )).await.map_err(|error| AgentError::Acp(error.to_string()))?;
    let text = text_buf.lock().await.remove(session_id).unwrap_or_default();
    Ok(AgentResponse { text, stop_reason: Some(format!("{:?}", response.stop_reason)) })
}

struct HeadlessClient { text: Arc<Mutex<HashMap<String, String>>> }

#[async_trait::async_trait(?Send)]
impl acp::Client for HeadlessClient {
    async fn request_permission(&self, args: acp::RequestPermissionRequest) -> acp::Result<acp::RequestPermissionResponse> {
        let option_id = args.options.iter()
            .find(|option| matches!(option.kind, acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways))
            .or_else(|| args.options.first())
            .map(|option| option.option_id.clone())
            .unwrap_or_else(|| acp::PermissionOptionId::new("allow"));
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(option_id))
        ))
    }
    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if let acp::SessionUpdate::AgentMessageChunk(chunk) = args.update
            && let acp::ContentBlock::Text(text) = chunk.content {
                self.text.lock().await.entry(args.session_id.to_string()).or_default().push_str(&text.text);
            }
        Ok(())
    }
}

pub struct AgentPool { agents: Mutex<HashMap<String, Arc<AgentHandle>>> }

impl AgentPool {
    pub fn new() -> Self { Self { agents: Mutex::new(HashMap::new()) } }
    pub async fn get_or_create(&self, agent_name: &str, command: &str, cwd: &Path) -> Arc<AgentHandle> {
        let mut agents = self.agents.lock().await;
        Arc::clone(agents.entry(agent_name.to_string())
            .or_insert_with(|| Arc::new(spawn_agent(command.to_string(), cwd.to_path_buf()))))
    }
    pub async fn close_all(&self) {
        for (_, handle) in self.agents.lock().await.drain() { let _ = handle.cmd_tx.send(AgentCommand::Shutdown); }
    }
}

impl Default for AgentPool { fn default() -> Self { Self::new() } }

pub struct AcpxDriver {
    command: String,
    cwd: PathBuf,
    pool: AgentPool,
    sessions: Mutex<HashMap<String, SessionRef>>,
}

impl std::fmt::Debug for AcpxDriver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AcpxDriver")
            .field("command", &self.command)
            .field("cwd", &self.cwd)
            .finish_non_exhaustive()
    }
}

struct SessionRef { handle: Arc<AgentHandle>, remote_id: String }

impl AcpxDriver {
    pub fn new(agent: String, _timeout: u64, cwd: PathBuf) -> Result<Self> {
        let command = AgentsConfig::load(&cwd).agents.get(&agent).map(|entry| entry.command.clone())
            .ok_or_else(|| AgentError::NotFound(format!("agent '{}' not configured in agents.json", agent)))?;
        Ok(Self { command, cwd, pool: AgentPool::new(), sessions: Mutex::new(HashMap::new()) })
    }
    pub async fn create_session(&self, name: &str) -> Result<()> {
        let handle = self.pool.get_or_create(&self.command, &self.command, &self.cwd).await;
        let remote_id = handle.new_session(SessionOpts {
            cwd: self.cwd.clone(), model: None, thought_level: None, allowed_tools: None, max_turns: None,
        }).await?;
        let previous = self.sessions.lock().await.insert(name.to_string(), SessionRef { handle: Arc::clone(&handle), remote_id });
        if let Some(previous) = previous { previous.handle.close_session(&previous.remote_id).await; }
        Ok(())
    }
    pub async fn prompt(&self, session: &str, prompt: &str) -> Result<AcpxResponse> {
        let sessions = self.sessions.lock().await;
        let current = sessions.get(session).ok_or_else(|| AgentError::Acp(format!("Unknown session '{session}'")))?;
        let (handle, remote_id) = (Arc::clone(&current.handle), current.remote_id.clone());
        drop(sessions);
        let response = handle.prompt(&remote_id, prompt).await?;
        Ok(AcpxResponse { text: response.text, tool_calls: Vec::new(), error: None })
    }
    pub async fn close_session(&self, name: &str) -> Result<()> {
        if let Some(session) = self.sessions.lock().await.remove(name) { session.handle.close_session(&session.remote_id).await; }
        Ok(())
    }
    pub async fn cancel(&self, session: &str) -> Result<()> {
        let sessions = self.sessions.lock().await;
        let current = sessions.get(session).ok_or_else(|| AgentError::Acp(format!("Unknown session '{session}'")))?;
        let (handle, remote_id) = (Arc::clone(&current.handle), current.remote_id.clone());
        drop(sessions);
        handle.cancel(&remote_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::{AcpxDriver, AgentError};
    use tempfile::tempdir;

    #[test]
    fn new_returns_not_found_when_agent_missing_from_config() {
        let tmp = tempdir().expect("tempdir");
        let error = AcpxDriver::new("missing-agent".to_string(), 0, tmp.path().to_path_buf()).expect_err("missing agent should error");
        assert!(matches!(error, AgentError::NotFound(message) if message.contains("missing-agent") && message.contains("agents.json")));
    }
}
