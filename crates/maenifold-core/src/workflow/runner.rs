use std::sync::Arc;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::acpx_driver::AcpxResponse;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::json;

use super::agent_session::{AgentSession, open_agent_session};
use super::output_parser::parse_output;
use super::output_types::WorkflowOutput;
use super::runner_helpers::{
    extract_workflow_session_id, parse_step_progress, step_think_config, terminal_workflow_status,
    workflow_prompt, workflow_text,
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
            thought_level: None,
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
            thought_level: None,
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

async fn prompt_workflow_agent(
    driver: &AgentSession,
    prompt: &str,
    step: &str,
    final_step: bool,
) -> Result<WorkflowAgentResponse> {
    let actual_prompt = workflow_prompt(prompt, step);
    let prompt_with_shape = format!(
        "{actual_prompt}\n\nWrap your response in <output> tags:\n{}\n\nYour thinking can appear outside the tags. Only the <output> block is parsed.",
        WorkflowOutput::expected_shape()
    );
    let response = driver.prompt(&prompt_with_shape).await?;
    let text = workflow_response_text(response)?;
    parse_workflow_with_retry(driver, &text, 3, final_step).await
}

struct WorkflowAgentResponse {
    output: WorkflowOutput,
    used_fallback: bool,
}

async fn parse_workflow_with_retry(
    driver: &AgentSession,
    initial_text: &str,
    max_retries: usize,
    final_step: bool,
) -> Result<WorkflowAgentResponse> {
    let mut text = initial_text.trim().to_string();

    for attempt in 0..=max_retries {
        match parse_workflow_output(&text) {
            Ok(output) => {
                return Ok(WorkflowAgentResponse {
                    output,
                    used_fallback: false,
                });
            }
            Err(error) if attempt < max_retries && !final_step => {
                let retry_prompt = workflow_retry_prompt(&error);
                let response = driver.prompt(&retry_prompt).await?;
                text = workflow_response_text(response)?.trim().to_string();
            }
            Err(error) => {
                eprintln!("workflow output parse failed after retries: {error}");
                return Ok(WorkflowAgentResponse {
                    output: WorkflowOutput {
                        response: text,
                        done: false,
                        conclusion: None,
                    },
                    used_fallback: true,
                });
            }
        }
    }

    Err(MaenifoldError::Message(
        "workflow parsing retry loop exhausted unexpectedly".to_string(),
    ))
}

fn parse_workflow_output(text: &str) -> std::result::Result<WorkflowOutput, String> {
    let parsed = parse_output(text).map_err(|error| error.to_string())?;
    let output = WorkflowOutput::from_parsed(&parsed).map_err(|error| error.to_string())?;
    let conclusion = output
        .conclusion
        .clone()
        .filter(|value| !value.trim().is_empty());
    if output.done && conclusion.is_none() {
        return Err("missing required <conclusion> when <done>true</done>".to_string());
    }
    Ok(WorkflowOutput {
        response: output.response,
        done: output.done,
        conclusion,
    })
}

fn workflow_response_text(response: AcpxResponse) -> Result<String> {
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text)
}

fn workflow_step_payload(
    session_id: &str,
    response: &WorkflowAgentResponse,
    final_step: bool,
) -> serde_json::Value {
    let response_text = workflow_response_text_for_step(response, final_step);
    if response.output.done || final_step {
        json!({
            "sessionId": session_id,
            "response": response_text,
            "status": "completed",
            "conclusion": workflow_conclusion_text(response, final_step)
        })
    } else {
        json!({ "sessionId": session_id, "response": response.output.response })
    }
}

fn workflow_completion_text(response: &WorkflowAgentResponse, final_step: bool) -> Option<String> {
    if response.output.done || final_step {
        Some(workflow_conclusion_text(response, final_step))
    } else {
        None
    }
}

fn workflow_conclusion_text(response: &WorkflowAgentResponse, final_step: bool) -> String {
    response
        .output
        .conclusion
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| workflow_response_text_for_step(response, final_step))
}

fn workflow_response_text_for_step(response: &WorkflowAgentResponse, final_step: bool) -> String {
    if response.used_fallback && final_step {
        fallback_step_tag(&response.output.response)
    } else {
        response.output.response.clone()
    }
}

fn workflow_retry_prompt(error: &str) -> String {
    format!(
        "Your response could not be parsed. Error: {error}\nExpected format:\n{}\n\nPlease rewrite your response with the correct <output> tags.",
        WorkflowOutput::expected_shape()
    )
}

fn fallback_step_tag(response: &str) -> String {
    if response.contains("[[") && response.contains("]]") {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}
