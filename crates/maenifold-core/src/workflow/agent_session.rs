use std::sync::Arc;

use crate::acp_config::AgentsConfig;
use crate::acp_native::{AcpxResponse, AgentError, AgentHandle, AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};

pub(crate) struct AgentSession {
    handle: Arc<AgentHandle>,
    remote_id: String,
}

pub(crate) async fn open_agent_session(
    pool: &AgentPool,
    agent_name: &str,
    _session_name: &str,
    opts: SessionOpts,
) -> Result<AgentSession> {
    let command = AgentsConfig::load(&opts.cwd)
        .agents
        .get(agent_name)
        .map(|entry| entry.command.clone())
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "agent '{}' not configured in agents.json",
                agent_name
            ))
        })?;
    let handle = pool.get_or_create(agent_name, &command, &opts.cwd).await;
    let remote_id = handle
        .new_session(opts)
        .await
        .map_err(agent_error)?
        .session_id;
    Ok(AgentSession { handle, remote_id })
}

impl AgentSession {
    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AcpxResponse> {
        let response = self
            .handle
            .prompt(&self.remote_id, prompt)
            .await
            .map_err(agent_error)?;
        Ok(AcpxResponse {
            text: response.text,
            tool_calls: response.tool_calls,
            thought: response.thought,
            error: None,
        })
    }

    pub(crate) async fn close(self) -> Result<()> {
        self.handle.close_session(&self.remote_id).await;
        Ok(())
    }
}

fn agent_error(error: AgentError) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}
