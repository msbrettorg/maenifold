use std::path::PathBuf;

use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use agent_client_protocol as acp;
use jockey::EventSender;
use serde_json::json;

use super::agent_session::open_agent_session;
use super::runner_helpers::{
    prompt_workflow_agent, step_think_config, terminal_workflow_status, workflow_completion_text,
    workflow_step_payload, workflow_text,
};
use super::runner_state::{current_step_index, init_workflow_session, workflow_total_steps};
use super::think_runner::{drive_think_session_with_driver, seed_think_session};

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
}

pub(crate) use super::think_runner::run_think_with_pool;
pub use super::think_runner::{ThinkOptions, ThinkResult};

pub(crate) fn append_close_error_context(
    primary: MaenifoldError,
    close_error: MaenifoldError,
) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "{primary}\nAdditionally, closing the workflow agent session failed: {close_error}"
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowResult {
    pub session_id: String,
    pub acp_session_id: Option<String>,
    pub conclusion: Option<String>,
    pub steps_completed: u32,
    pub status: WorkflowRunStatus,
}

fn workflow_failure_result(
    session_id: impl Into<String>,
    acp_session_id: impl Into<String>,
    error: &MaenifoldError,
) -> WorkflowResult {
    WorkflowResult {
        session_id: session_id.into(),
        acp_session_id: Some(acp_session_id.into()),
        conclusion: None,
        steps_completed: 0,
        status: WorkflowRunStatus::Failed(error.to_string()),
    }
}

/// Shared state carried through the workflow run loop.
struct RunContext<'a> {
    tool_context: &'a ToolContext,
    session_id: String,
    event_tx: EventSender,
    model: Option<String>,
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

/// Like [`run_workflow_with_pool`] but accepts a fully constructed agent session config,
/// forwarding all session configuration (model, mode, thought_level, allowed_tools, etc.)
/// instead of building a stripped-down one internally.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_workflow_with_pool_session_opts(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    agent: &str,
    prompt: &str,
    session_config: AgentSessionConfig,
    event_tx: EventSender,
    keep_session_alive: bool,
) -> Result<WorkflowResult> {
    let (session_id, start_text) =
        init_workflow_session(context, workflow_id, existing_session_id)?;
    if trace_enabled() {
        eprintln!("[workflow] init session");
    }
    let run_ctx = RunContext {
        tool_context: context,
        session_id,
        event_tx,
        model: session_config.model.clone(),
    };
    let driver = open_agent_session(agent, &session_config, &run_ctx.event_tx).await?;
    let mut step = workflow_text(&start_text);
    let total_steps = workflow_total_steps(context, workflow_id, &step)?;
    let mut step_index = current_step_index(context, &run_ctx.session_id, &step);
    let mut steps_completed = 0u32;
    let mut conclusion = None;

    let result: Result<WorkflowResult> = async {
        loop {
            if let Some(status) = terminal_workflow_status(&step) {
                if trace_enabled() {
                    eprintln!("[workflow] complete");
                }
                break Ok(WorkflowResult {
                    session_id: run_ctx.session_id.clone(),
                    acp_session_id: Some(driver.remote_id().to_string()),
                    conclusion,
                    steps_completed,
                    status,
                });
            }
            if step.starts_with("ERROR:") {
                break Err(MaenifoldError::Message(step));
            }

            if trace_enabled() {
                eprintln!("[workflow] step {}/{}", step_index, total_steps);
            }
            maybe_run_step_thinking(&run_ctx, &driver, &step).await?;
            let final_step = step_index >= total_steps;
            let response = prompt_workflow_agent(&driver, prompt, &step, None).await?;
            if let Some(value) = workflow_completion_text(&response, final_step) {
                conclusion = Some(value);
            }

            let payload = workflow_step_payload(&run_ctx.session_id, &response, final_step);
            step = workflow_text(&super::handler::workflow(context, &payload)?);
            steps_completed += 1;
            step_index = step_index.saturating_add(1);
        }
    }
    .await;
    let acp_session_id = driver.remote_id().to_string();
    let failure_result = |error: &MaenifoldError| {
        workflow_failure_result(run_ctx.session_id.clone(), acp_session_id.clone(), error)
    };
    if keep_session_alive {
        driver.keep_alive();
        match result {
            Ok(value) => Ok(value),
            Err(error) => Ok(failure_result(&error)),
        }
    } else {
        let close_result = driver.close().await;
        match (result, close_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(close_error)) => {
                let error = append_close_error_context(error, close_error);
                Ok(failure_result(&error))
            }
            (Err(error), Ok(())) => Ok(failure_result(&error)),
        }
    }
}

async fn maybe_run_step_thinking(
    run_ctx: &RunContext<'_>,
    driver: &super::agent_session::AgentSession,
    step: &str,
) -> Result<()> {
    let Some((topic, total_thoughts)) = step_think_config(step) else {
        return Ok(());
    };
    let think_opts = ThinkOptions {
        total_thoughts,
        parent_workflow_id: Some(run_ctx.session_id.clone()),
        model: run_ctx.model.clone(),
    };
    let think_session_id = seed_think_session(run_ctx.tool_context, &topic, &think_opts).await?;
    let _ = super::handler::workflow(
        run_ctx.tool_context,
        &json!({
            "sessionId": run_ctx.session_id,
            "response": "Spawning [[thinking]] submachine",
            "submachineSessionId": &think_session_id
        }),
    )?;
    let _ = drive_think_session_with_driver(
        run_ctx.tool_context,
        driver,
        &think_session_id,
        &topic,
        "",
        &think_opts,
        None,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AgentSessionConfig, WorkflowRunStatus, append_close_error_context,
        is_reentrant_workflow_tool, sanitize_workflow_session_config, workflow_failure_result,
    };
    use crate::error::MaenifoldError;
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

    #[test]
    fn workflow_failure_result_preserves_session_provenance() {
        let result = workflow_failure_result(
            "workflow-123",
            "acp-456",
            &MaenifoldError::Message("prompt failed".to_string()),
        );

        assert_eq!(result.session_id, "workflow-123");
        assert_eq!(result.acp_session_id.as_deref(), Some("acp-456"));
        assert_eq!(result.conclusion, None);
        assert_eq!(result.steps_completed, 0);
        assert_eq!(
            result.status,
            WorkflowRunStatus::Failed("prompt failed".to_string())
        );
    }
}
