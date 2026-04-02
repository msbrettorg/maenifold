use std::sync::Arc;

use crate::acp_config::AgentsConfig;
use crate::acp_native::{AcpxResponse, AgentError, AgentHandle, AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use agent_client_protocol as acp;

pub(crate) struct AgentSession {
    handle: Arc<AgentHandle>,
    remote_id: String,
}

pub(crate) async fn open_agent_session(
    pool: &AgentPool,
    agent_name: &str,
    _session_name: &str,
    mut opts: SessionOpts,
) -> Result<AgentSession> {
    let config = AgentsConfig::load(&opts.cwd);
    let config_entry = config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "agent '{}' not configured in agents.json",
            agent_name
        ))
    })?;
    let command = config_entry.command.clone();
    let env = config_entry.env.clone();
    if opts.mcp_servers.is_empty() && !config_entry.mcp_servers.is_empty() {
        opts.mcp_servers = config_entry
            .mcp_servers
            .iter()
            .map(|s| s.to_acp())
            .collect();
    }
    let handle = pool
        .get_or_create(agent_name, &command, &opts.cwd, env)
        .await;
    let remote_id = handle
        .new_session(opts)
        .await
        .map_err(agent_error)?
        .session_id;
    Ok(AgentSession { handle, remote_id })
}

impl AgentSession {
    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AcpxResponse> {
        self.prompt_with_context(prompt, vec![]).await
    }

    pub(crate) async fn prompt_with_context(
        &self,
        prompt: &str,
        context: Vec<acp::ContentBlock>,
    ) -> Result<AcpxResponse> {
        let response = self
            .handle
            .prompt_with_context(&self.remote_id, prompt, context)
            .await
            .map_err(agent_error)?;
        let error = response.stop_reason.as_deref().and_then(|sr| match sr {
            "end_turn" | "max_tokens" | "max_turn_requests" | "refusal" | "cancelled" => None,
            other => Some(other.to_string()),
        });
        Ok(AcpxResponse {
            text: response.text,
            tool_calls: response.tool_calls,
            thought: response.thought,
            error,
            usage: response.usage,
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
