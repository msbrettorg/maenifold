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

use super::DEFAULT_SESSION_TIMEOUT_SECONDS;

pub type Result<T> = std::result::Result<T, AgentError>;
pub type AcpxError = AgentError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("ACP error: {0}")]
    Acp(String),
    #[error("Agent process failed to start: {0}")]
    Spawn(String),
    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Channel closed")]
    ChannelClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOpts {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub thought_level: Option<String>,
    pub config_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub mcp_servers: Vec<acp::McpServer>,
}

impl SessionOpts {
    pub fn resolved_timeout_seconds(&self) -> u64 {
        self.timeout_seconds
            .unwrap_or(DEFAULT_SESSION_TIMEOUT_SECONDS)
    }
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

pub struct AgentResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub thought: Option<String>,
    pub stop_reason: Option<String>,
    pub usage: Option<AcpxUsage>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcpxResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub thought: Option<String>,
    pub error: Option<String>,
    pub usage: Option<AcpxUsage>,
}

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
}

pub enum AgentCommand {
    NewSession {
        opts: SessionOpts,
        reply: oneshot::Sender<Result<SessionInfo>>,
    },
    ListSessions {
        cwd: Option<PathBuf>,
        reply: oneshot::Sender<Result<Vec<AcpxSessionInfo>>>,
    },
    LoadSession {
        session_id: String,
        opts: SessionOpts,
        reply: oneshot::Sender<Result<SessionInfo>>,
    },
    Prompt {
        session_id: String,
        prompt: String,
        reply: oneshot::Sender<Result<AgentResponse>>,
    },
    Cancel {
        session_id: String,
        reply: oneshot::Sender<Result<()>>,
    },
    CloseSession {
        session_id: String,
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
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(AgentCommand::NewSession { opts, reply: tx })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn list_sessions(&self, cwd: Option<PathBuf>) -> Result<Vec<AcpxSessionInfo>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(AgentCommand::ListSessions { cwd, reply: tx })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn load_session(&self, session_id: &str, opts: SessionOpts) -> Result<SessionInfo> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(AgentCommand::LoadSession {
                session_id: session_id.to_string(),
                opts,
                reply: tx,
            })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn prompt(&self, session_id: &str, prompt: &str) -> Result<AgentResponse> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(AgentCommand::Prompt {
                session_id: session_id.to_string(),
                prompt: prompt.to_string(),
                reply: tx,
            })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn cancel(&self, session_id: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(AgentCommand::Cancel {
                session_id: session_id.to_string(),
                reply: tx,
            })
            .map_err(|_| AgentError::ChannelClosed)?;
        rx.await.map_err(|_| AgentError::ChannelClosed)?
    }
    pub async fn close_session(&self, session_id: &str) {
        let _ = self.cmd_tx.send(AgentCommand::CloseSession {
            session_id: session_id.to_string(),
        });
    }
}
