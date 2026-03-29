use std::sync::Arc;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::json;

use super::agent_session::{AgentSession, open_agent_session};
use super::runner_helpers::{
    extract_workflow_session_id, parse_step_progress, prompt_workflow_agent, step_think_config,
    terminal_workflow_status, workflow_completion_text, workflow_prompt, workflow_step_payload,
    workflow_text,
};
use super::think_runner::{drive_think_session, seed_think_session};

pub(crate) use super::think_runner::run_think_with_pool;
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
    run_workflow_with_pool(
        context,
        workflow_id,
        agent,
        prompt,
        None,
        Arc::new(AgentPool::new()),
    )
    .await
}

pub(crate) async fn run_workflow_with_pool(
    context: &ToolContext,
    workflow_id: &str,
    agent: &str,
    prompt: &str,
    timeout_seconds: Option<u64>,
    pool: Arc<AgentPool>,
) -> Result<WorkflowResult> {
    let start = super::handler::workflow(context, &json!({ "workflowId": workflow_id }))?;
    let session_id = extract_workflow_session_id(&start).ok_or_else(|| {
        MaenifoldError::Message("Workflow session ID missing from response".to_string())
    })?;
    let driver = open_agent_session(
        &pool,
        agent,
        &session_id,
        SessionOpts {
            cwd: context.config.maenifold_root.clone(),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds,
            mcp_servers: vec![],
        },
    )
    .await?;
    let mut step = workflow_text(&start);
    let mut steps_completed = 0u32;
    let mut conclusion = None;

    let result: Result<WorkflowResult> = async {
        loop {
            if let Some(status) = terminal_workflow_status(&step) {
                break Ok(WorkflowResult {
                    session_id,
                    conclusion,
                    steps_completed,
                    status,
                });
            }
            if step.starts_with("ERROR:") {
                break Err(MaenifoldError::Message(step));
            }

            maybe_run_step_thinking(context, agent, &session_id, &step, pool.as_ref()).await?;
            let final_step =
                matches!(parse_step_progress(&step), Some((current, total)) if current == total);
            let response = prompt_workflow_agent(&driver, prompt, &step, final_step).await?;
            if let Some(value) = workflow_completion_text(&response, final_step) {
                conclusion = Some(value);
            }

            let payload = workflow_step_payload(&session_id, &response, final_step);
            step = workflow_text(&super::handler::workflow(context, &payload)?);
            steps_completed += 1;
        }
    }
    .await;
    let close_result = driver.close().await;
    match (result, close_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
    }
}

async fn maybe_run_step_thinking(
    context: &ToolContext,
    agent: &str,
    workflow_session_id: &str,
    step: &str,
    pool: &AgentPool,
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
    let _ = drive_think_session(
        context,
        agent,
        &think_session_id,
        &topic,
        &think_opts,
        SessionOpts {
            cwd: context.config.maenifold_root.clone(),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: None,
            mcp_servers: vec![],
        },
        pool,
    )
    .await?;
    Ok(())
}
