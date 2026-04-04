/// ACP native client types — errors, options, responses, commands, and handle.
use agent_client_protocol as acp;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{mpsc, oneshot};

pub type Result<T> = std::result::Result<T, AgentError>;
pub type AcpxError = AgentError;
type Reply<T> = oneshot::Sender<Result<T>>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("ACP error: {0}")]
    Acp(String),
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
    pub cwd: PathBuf,
    pub auto_grant_permissions: bool,
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
    Cancel {
        session_id: String,
        reply: Reply<()>,
    },
    CloseSession {
        session_id: String,
        reply: Reply<()>,
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
    pub(crate) cmd_tx: mpsc::UnboundedSender<AgentCommand>,
    pub(super) init: Arc<StdMutex<Option<InitInfo>>>,
    pub(super) alive: Arc<AtomicBool>,
}

impl AgentHandle {
    async fn request<T>(&self, command: impl FnOnce(Reply<T>) -> AgentCommand) -> Result<T> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(command(reply))
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
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
    pub async fn new_session(&self, opts: SessionOpts) -> Result<SessionInfo> {
        self.request(|reply| AgentCommand::NewSession { opts, reply })
            .await
    }
    pub async fn list_sessions(&self, cwd: Option<PathBuf>) -> Result<Vec<AcpxSessionInfo>> {
        self.request(|reply| AgentCommand::ListSessions { cwd, reply })
            .await
    }
    pub async fn load_session(&self, session_id: &str, opts: SessionOpts) -> Result<SessionInfo> {
        self.request(|reply| AgentCommand::LoadSession {
            session_id: session_id.to_string(),
            opts,
            reply,
        })
        .await
    }
    pub async fn prompt(&self, session_id: &str, prompt: &str) -> Result<AgentResponse> {
        self.request(|reply| AgentCommand::Prompt {
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
        self.request(|reply| AgentCommand::PromptWithContext {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
            context_blocks,
            reply,
        })
        .await
    }
    pub async fn cancel(&self, session_id: &str) -> Result<()> {
        self.request(|reply| AgentCommand::Cancel {
            session_id: session_id.to_string(),
            reply,
        })
        .await
    }
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        self.request(|reply| AgentCommand::CloseSession {
            session_id: session_id.to_string(),
            reply,
        })
        .await
    }
}
