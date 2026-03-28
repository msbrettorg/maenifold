use std::{path::Path, sync::Arc};

use crate::acp_config::AgentsConfig;
use crate::acp_native::{AcpxResponse, AgentError, AgentHandle, AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};

pub(crate) struct AgentSession { handle: Arc<AgentHandle>, remote_id: String }

pub(crate) async fn open_agent_session(
    pool: &AgentPool,
    cwd: &Path,
    agent_name: &str,
    _session_name: &str,
) -> Result<AgentSession> {
    let command = AgentsConfig::load(cwd)
        .agents
        .get(agent_name)
        .map(|entry| entry.command.clone())
        .ok_or_else(|| MaenifoldError::Message(format!("agent '{}' not configured in agents.json", agent_name)))?;
    let handle = pool.get_or_create(agent_name, &command, cwd).await;
    let remote_id = handle.new_session(SessionOpts {
        cwd: cwd.to_path_buf(), model: None, thought_level: None, allowed_tools: None, max_turns: None,
    }).await.map_err(agent_error)?;
    Ok(AgentSession { handle, remote_id })
}

impl AgentSession {
    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AcpxResponse> {
        let response = self.handle.prompt(&self.remote_id, prompt).await.map_err(agent_error)?;
        Ok(AcpxResponse { text: response.text, tool_calls: Vec::new(), error: None })
    }

    pub(crate) async fn close(self) -> Result<()> {
        self.handle.close_session(&self.remote_id).await;
        Ok(())
    }
}

fn agent_error(error: AgentError) -> MaenifoldError { MaenifoldError::Message(error.to_string()) }
