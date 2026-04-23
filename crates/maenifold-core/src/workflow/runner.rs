use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use jockey::EventSender;
use serde_json::json;

use super::adoption::require_adopted_role;
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

#[allow(unused_imports)]
pub(crate) use super::runner_prompt::{
    AgentSessionConfig, append_close_error_context, is_reentrant_workflow_tool,
    run_prompt_with_pool, run_prompt_with_pool_context, sanitize_workflow_session_config,
};
pub(crate) use super::think_runner::run_think_with_pool;
pub use super::think_runner::{ThinkOptions, ThinkResult};

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
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    event_tx: EventSender,
    keep_session_alive: bool,
) -> Result<WorkflowResult> {
    require_adopted_role("workflow invocation", requires_role, adopt_role)?;
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
    for instruction in adopt_instructions {
        let response = driver.prompt(instruction).await?;
        if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
            return Err(MaenifoldError::Message(error));
        }
    }
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
                    acp_session_id: Some(driver.remote_id()),
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
    let acp_session_id = driver.remote_id();
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
    use super::{WorkflowRunStatus, workflow_failure_result};
    use crate::error::MaenifoldError;

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
