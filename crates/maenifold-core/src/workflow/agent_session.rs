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
        MaenifoldError::AgentSetup(format!(
            "agent '{}' not configured in maenifold.json",
            agent_name
        ))
    })?;
    let command = config_entry.command.clone();
    let env = config_entry.env.clone();
    if opts.mcp_servers.is_empty() {
        let mut servers: Vec<_> = config.mcp_servers.iter().map(|s| s.to_acp()).collect();
        servers.extend(config_entry.mcp_servers.iter().map(|s| s.to_acp()));
        opts.mcp_servers = servers;
    }
    let handle = pool
        .get_or_create(
            agent_name,
            &command,
            &opts.cwd,
            env,
            opts.auto_grant_permissions,
        )
        .await;
    let remote_id = handle
        .new_session(opts)
        .await
        .map_err(session_setup_error)?
        .session_id;
    Ok(AgentSession { handle, remote_id })
}

impl AgentSession {
    pub(crate) fn remote_id(&self) -> &str {
        &self.remote_id
    }

    pub(crate) fn keep_alive(self) {}

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
            .map_err(prompt_execution_error)?;
        Ok(response)
    }

    pub(crate) async fn close(self) -> Result<()> {
        if let Err(error) = self.handle.close_session(&self.remote_id).await {
            tracing::warn!(
                session_id = %self.remote_id,
                error = %error,
                "Failed to close ACP session"
            );
        }
        Ok(())
    }
}

fn session_setup_error(error: AgentError) -> MaenifoldError {
    MaenifoldError::AgentSetup(error.to_string())
}

fn prompt_execution_error(error: AgentError) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::acp_native::test_handle_for_tests;

    use super::*;

    #[tokio::test]
    async fn keep_alive_preserves_tracked_session_without_queue_traffic() {
        let (handle, mut request_rx, mut control_rx) = test_handle_for_tests(1);
        handle.register_test_session("session-keep-alive").await;
        let handle = Arc::new(handle);
        let session = AgentSession {
            handle: Arc::clone(&handle),
            remote_id: "session-keep-alive".to_string(),
        };

        session.keep_alive();

        assert!(
            request_rx.try_recv().is_err(),
            "keep_alive should not enqueue request commands"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "keep_alive should not enqueue control commands"
        );
        assert!(
            handle.has_tracked_session("session-keep-alive").await,
            "keep_alive should preserve tracked ACP session state"
        );
    }
}
