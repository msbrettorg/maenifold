use std::path::PathBuf;

use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use agent_client_protocol as acp;
use jockey::EventSender;

use super::agent_session::open_agent_session;

// T-01.15: extracted from runner.rs to keep that module under the LOC budget.

pub(crate) fn append_close_error_context(
    primary: MaenifoldError,
    close_error: MaenifoldError,
) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "{primary}\nAdditionally, closing the workflow agent session failed: {close_error}"
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AgentSessionConfig {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub mode: Option<String>,
    pub thought_level: Option<String>,
    pub config_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<u32>,
    pub timeout_seconds: Option<u64>,
    /// Whether permission requests should be auto-granted.
    /// Defaults to `false` (safe for interactive sessions).
    /// Pipeline/headless execution paths should set this to `true`.
    pub auto_approve: bool,
}

pub(crate) fn sanitize_workflow_session_config(
    mut config: AgentSessionConfig,
) -> AgentSessionConfig {
    let allowed_tools = config.allowed_tools.take().unwrap_or_else(|| {
        ToolRegistry::global()
            .canonical_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    });
    config.allowed_tools = Some(
        allowed_tools
            .into_iter()
            .filter(|tool| !is_reentrant_workflow_tool(tool))
            .collect(),
    );
    config
}

pub(crate) fn is_reentrant_workflow_tool(tool: &str) -> bool {
    matches!(
        tool.to_ascii_lowercase().as_str(),
        "workflow" | "sequentialthinking" | "pipeline" | "acpexec" | "acp_exec"
    )
}

/// Send a single prompt to an agent using a specific event stream.
#[allow(dead_code)]
pub(crate) async fn run_prompt_with_pool(
    event_tx: &EventSender,
    agent: &str,
    prompt: &str,
    session_config: AgentSessionConfig,
) -> Result<String> {
    run_prompt_with_pool_context(event_tx, agent, prompt, None, session_config).await
}

/// Send a single prompt with optional embedded context blocks using a specific event stream.
pub(crate) async fn run_prompt_with_pool_context(
    event_tx: &EventSender,
    agent: &str,
    prompt: &str,
    context: Option<Vec<acp::ContentBlock>>,
    session_config: AgentSessionConfig,
) -> Result<String> {
    let session = open_agent_session(agent, &session_config, event_tx).await?;
    let prompt = context
        .map(|blocks| super::runner_helpers::prompt_with_embedded_context(prompt, blocks))
        .unwrap_or_else(|| prompt.to_string());
    let response_result = session.prompt(&prompt).await;
    let close_result = session.close().await;
    let prompt_result = match response_result {
        Ok(response) => {
            if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
                Err(MaenifoldError::Message(error))
            } else {
                Ok(response.text.trim().to_string())
            }
        }
        Err(error) => Err(error),
    };
    match (prompt_result, close_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(close_error)) => Err(append_close_error_context(error, close_error)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentSessionConfig, append_close_error_context, is_reentrant_workflow_tool,
        sanitize_workflow_session_config,
    };
    use std::path::PathBuf;

    fn session_config(allowed_tools: Option<Vec<&str>>) -> AgentSessionConfig {
        AgentSessionConfig {
            cwd: PathBuf::from("."),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: allowed_tools.map(|tools| {
                tools
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect()
            }),
            max_turns: None,
            timeout_seconds: None,
            session_id: None,
            auto_approve: true,
        }
    }

    #[test]
    fn sanitize_workflow_session_config_filters_reentrant_tools_from_explicit_allowlist() {
        let sanitized = sanitize_workflow_session_config(session_config(Some(vec![
            "ReadMemory",
            "workflow",
            "SequentialThinking",
        ])));

        assert_eq!(
            sanitized.allowed_tools,
            Some(vec!["ReadMemory".to_string()])
        );
    }

    #[test]
    fn sanitize_workflow_session_config_defaults_to_registry_without_reentrant_tools() {
        let sanitized = sanitize_workflow_session_config(session_config(None));
        let allowed_tools = sanitized
            .allowed_tools
            .expect("workflow sessions should always have an explicit allowlist");

        assert!(allowed_tools.iter().any(|tool| tool == "ReadMemory"));
        assert!(
            allowed_tools
                .iter()
                .all(|tool| !is_reentrant_workflow_tool(tool))
        );
    }

    #[test]
    fn append_close_error_context_preserves_primary_error_and_close_context() {
        let combined = append_close_error_context(
            crate::error::MaenifoldError::Message("primary failure".to_string()),
            crate::error::MaenifoldError::Message("close failure".to_string()),
        );

        assert_eq!(
            combined.to_string(),
            "primary failure\nAdditionally, closing the workflow agent session failed: close failure"
        );
    }
}
