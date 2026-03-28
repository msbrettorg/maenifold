use crate::acpx_driver::AcpxDriver;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::json;

use super::runner_helpers::{
    extract_workflow_session_id, parse_step_progress, step_think_config, tag_response,
    terminal_workflow_status, workflow_prompt, workflow_text,
};
use super::think_runner::{drive_think_session, seed_think_session};

pub use super::think_runner::{ThinkOptions, ThinkResult, run_think};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowResult {
    pub session_id: String,
    pub conclusion: Option<String>,
    pub steps_completed: u32,
    pub status: WorkflowRunStatus,
}

pub async fn run_workflow(
    context: &ToolContext,
    workflow_id: &str,
    agent: &str,
    prompt: &str,
) -> Result<WorkflowResult> {
    let start = super::handler::workflow(context, &json!({ "workflowId": workflow_id }))?;
    let session_id = extract_workflow_session_id(&start).ok_or_else(|| {
        MaenifoldError::Message("Workflow session ID missing from response".to_string())
    })?;
    let driver = create_workflow_driver(context, agent, &session_id).await?;
    let mut step = workflow_text(&start);
    let mut steps_completed = 0u32;
    let mut conclusion = None;

    loop {
        if let Some(status) = terminal_workflow_status(&step) {
            return Ok(WorkflowResult {
                session_id,
                conclusion,
                steps_completed,
                status,
            });
        }
        if step.starts_with("ERROR:") {
            return Err(MaenifoldError::Message(step));
        }

        maybe_run_step_thinking(context, agent, &session_id, &step).await?;
        let response = prompt_workflow_agent(&driver, &session_id, prompt, &step).await?;
        let final_step =
            matches!(parse_step_progress(&step), Some((current, total)) if current == total);
        if final_step {
            conclusion = Some(response.clone());
        }

        let payload = workflow_step_payload(&session_id, &response, final_step);
        step = workflow_text(&super::handler::workflow(context, &payload)?);
        steps_completed += 1;
    }
}

async fn create_workflow_driver(
    context: &ToolContext,
    agent: &str,
    session_id: &str,
) -> Result<AcpxDriver> {
    let driver = AcpxDriver::new(agent.to_string(), 0, context.config.maenifold_root.clone())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    driver
        .create_session(session_id)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(driver)
}

async fn maybe_run_step_thinking(
    context: &ToolContext,
    agent: &str,
    workflow_session_id: &str,
    step: &str,
) -> Result<()> {
    let Some((topic, total_thoughts)) = step_think_config(step) else {
        return Ok(());
    };

    let think_opts = ThinkOptions {
        total_thoughts,
        parent_workflow_id: Some(workflow_session_id.to_string()),
    };
    let think_session_id = seed_think_session(context, &topic, &think_opts).await?;
    let _ = super::handler::workflow(
        context,
        &json!({
            "sessionId": workflow_session_id,
            "response": "Spawning [[thinking]] submachine",
            "submachineSessionId": &think_session_id
        }),
    )?;
    let _ = drive_think_session(context, agent, &think_session_id, &topic, &think_opts).await?;
    Ok(())
}

async fn prompt_workflow_agent(
    driver: &AcpxDriver,
    session_id: &str,
    prompt: &str,
    step: &str,
) -> Result<String> {
    let response = driver
        .prompt(session_id, &workflow_prompt(prompt, step))
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(tag_response(response.text.trim()))
}

fn workflow_step_payload(session_id: &str, response: &str, final_step: bool) -> serde_json::Value {
    if final_step {
        json!({
            "sessionId": session_id,
            "response": response,
            "status": "completed",
            "conclusion": response
        })
    } else {
        json!({ "sessionId": session_id, "response": response })
    }
}
