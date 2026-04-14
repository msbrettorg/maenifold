use jockey::acp::AcpPromptResult;

use crate::acp_config::AgentsConfig;
use crate::error::{MaenifoldError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentResponse {
    pub text: String,
    pub error: Option<String>,
}

impl From<AcpPromptResult> for AgentResponse {
    fn from(result: AcpPromptResult) -> Self {
        let error = if result.ok {
            None
        } else if !result.output.trim().is_empty() {
            Some(result.output.clone())
        } else {
            result.error_code.clone()
        };

        Self {
            text: result.output,
            error,
        }
    }
}

pub(crate) struct AgentSession {
    agent_name: String,
    runtime_kind: String,
    session_id: String,
    cwd: String,
    auto_approve: bool,
    event_tx: jockey::EventSender,
}

pub(crate) async fn open_agent_session(
    agent_name: &str,
    cwd: &std::path::Path,
    event_tx: &jockey::EventSender,
) -> Result<AgentSession> {
    let cwd_str = cwd.to_string_lossy().to_string();
    let runtime_kind = resolve_runtime_kind(agent_name, &cwd_str)?;
    let auto_approve = true;
    let (_config_options, _modes) =
        jockey::acp::prewarm_role_for_config(&runtime_kind, agent_name, &cwd_str, None).await;

    let session_id = uuid::Uuid::new_v4().to_string();

    Ok(AgentSession {
        agent_name: agent_name.to_string(),
        runtime_kind,
        session_id,
        cwd: cwd_str,
        auto_approve,
        event_tx: event_tx.clone(),
    })
}

impl AgentSession {
    pub(crate) fn remote_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn keep_alive(&self) {}

    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AgentResponse> {
        let result = jockey::acp::execute_runtime(
            &self.runtime_kind,
            &self.agent_name,
            prompt,
            &[],
            &self.cwd,
            &self.event_tx,
            self.auto_approve,
            vec![],
            None,
            vec![],
            None,
            &self.session_id,
        )
        .await;

        Ok(result.into())
    }

    pub(crate) async fn close(self) -> Result<()> {
        jockey::acp::reset_session(
            &self.runtime_kind,
            &self.agent_name,
            Some(self.session_id.as_str()),
        )
        .await
        .map_err(MaenifoldError::Message)?;
        Ok(())
    }
}

fn resolve_runtime_kind(agent_name: &str, cwd: &str) -> Result<String> {
    let config = AgentsConfig::load(std::path::Path::new(cwd));
    config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::AgentSetup(format!(
            "agent '{agent_name}' not configured in maenifold.json"
        ))
    })?;
    Ok(agent_name.to_string())
}
