use std::sync::Arc;

use crate::acp_native::{AgentPool, GLOBAL_AGENT_POOL, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use agent_client_protocol as acp;
use serde_json::json;

use super::agent_session::open_agent_session;
use super::runner_helpers::{
    parse_step_progress, prompt_workflow_agent, step_think_config, terminal_workflow_status,
    workflow_completion_text, workflow_step_payload, workflow_text,
};
use super::think_runner::{drive_think_session, seed_think_session};
use super::{frontmatter_usize, read_workflow_definition, workflow_steps};

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

/// Shared state carried through the workflow run loop.
struct RunContext<'a> {
    tool_context: &'a ToolContext,
    agent: &'a str,
    session_id: String,
    pool: Arc<AgentPool>,
    model: Option<String>,
}

impl RunContext<'_> {
    fn default_session_opts(&self) -> SessionOpts {
        SessionOpts {
            cwd: self.tool_context.config.maenifold_root.clone(),
            model: self.model.clone(),
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: None,
            mcp_servers: vec![],
        }
    }
}

/// Send a single prompt to an ACP agent and return the response text.
/// Uses the global pool for agent reuse.
pub async fn run_prompt(agent: &str, prompt: &str, opts: SessionOpts) -> Result<String> {
    run_prompt_with_pool(&GLOBAL_AGENT_POOL, agent, prompt, opts).await
}

/// Send a single prompt to an ACP agent using a specific pool.
pub(crate) async fn run_prompt_with_pool(
    pool: &AgentPool,
    agent: &str,
    prompt: &str,
    opts: SessionOpts,
) -> Result<String> {
    run_prompt_with_pool_context(pool, agent, prompt, None, opts).await
}

/// Send a single prompt with optional embedded context blocks using a specific pool.
pub(crate) async fn run_prompt_with_pool_context(
    pool: &AgentPool,
    agent: &str,
    prompt: &str,
    context: Option<Vec<acp::ContentBlock>>,
    opts: SessionOpts,
) -> Result<String> {
    let session = open_agent_session(pool, agent, "prompt", opts).await?;
    let response = match context {
        Some(blocks) => session.prompt_with_context(prompt, blocks).await,
        None => session.prompt(prompt).await,
    };
    let _ = session.close().await;
    let response = response?;
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text.trim().to_string())
}

pub async fn run_workflow(
    context: &ToolContext,
    workflow_id: &str,
    agent: &str,
    prompt: &str,
    model: Option<String>,
) -> Result<WorkflowResult> {
    run_workflow_with_pool(
        context,
        workflow_id,
        None,
        agent,
        prompt,
        None,
        model,
        None,
        Arc::clone(&GLOBAL_AGENT_POOL),
    )
    .await
}

pub(crate) async fn run_workflow_with_pool(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    agent: &str,
    prompt: &str,
    timeout_seconds: Option<u64>,
    model: Option<String>,
    content_blocks: Option<Vec<acp::ContentBlock>>,
    pool: Arc<AgentPool>,
) -> Result<WorkflowResult> {
    let (session_id, start_text) =
        init_workflow_session(context, workflow_id, existing_session_id)?;
    let run_ctx = RunContext {
        tool_context: context,
        agent,
        session_id,
        pool,
        model,
    };
    let mut driver_opts = run_ctx.default_session_opts();
    driver_opts.timeout_seconds = timeout_seconds;
    let driver = open_agent_session(&run_ctx.pool, agent, &run_ctx.session_id, driver_opts).await?;
    let mut step = workflow_text(&start_text);
    let total_steps = workflow_total_steps(context, workflow_id, &step)?;
    let mut step_index = current_step_index(context, &run_ctx.session_id, &step);
    let mut steps_completed = 0u32;
    let mut conclusion = None;
    let mut first_context = content_blocks;

    let result: Result<WorkflowResult> = async {
        loop {
            if let Some(status) = terminal_workflow_status(&step) {
                break Ok(WorkflowResult {
                    session_id: run_ctx.session_id.clone(),
                    conclusion,
                    steps_completed,
                    status,
                });
            }
            if step.starts_with("ERROR:") {
                break Err(MaenifoldError::Message(step));
            }

            maybe_run_step_thinking(&run_ctx, &step).await?;
            let final_step = step_index >= total_steps;
            let response =
                prompt_workflow_agent(&driver, prompt, &step, final_step, first_context.take())
                    .await?;
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
    let close_result = driver.close().await;
    match (result, close_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
    }
}

/// Like [`run_workflow_with_pool`] but accepts a fully constructed [`SessionOpts`] directly,
/// forwarding all session configuration (model, mode, thought_level, allowed_tools, etc.)
/// instead of building a stripped-down one internally.
pub(crate) async fn run_workflow_with_pool_session_opts(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    agent: &str,
    prompt: &str,
    session_opts: SessionOpts,
    pool: Arc<AgentPool>,
) -> Result<WorkflowResult> {
    let (session_id, start_text) =
        init_workflow_session(context, workflow_id, existing_session_id)?;
    let run_ctx = RunContext {
        tool_context: context,
        agent,
        session_id,
        pool,
        model: session_opts.model.clone(),
    };
    let driver =
        open_agent_session(&run_ctx.pool, agent, &run_ctx.session_id, session_opts).await?;
    let mut step = workflow_text(&start_text);
    let total_steps = workflow_total_steps(context, workflow_id, &step)?;
    let mut step_index = current_step_index(context, &run_ctx.session_id, &step);
    let mut steps_completed = 0u32;
    let mut conclusion = None;

    let result: Result<WorkflowResult> = async {
        loop {
            if let Some(status) = terminal_workflow_status(&step) {
                break Ok(WorkflowResult {
                    session_id: run_ctx.session_id.clone(),
                    conclusion,
                    steps_completed,
                    status,
                });
            }
            if step.starts_with("ERROR:") {
                break Err(MaenifoldError::Message(step));
            }

            maybe_run_step_thinking(&run_ctx, &step).await?;
            let final_step = step_index >= total_steps;
            let response = prompt_workflow_agent(&driver, prompt, &step, final_step, None).await?;
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
    let close_result = driver.close().await;
    match (result, close_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
    }
}

/// Initializes a workflow session, returning `(session_id, handler_output)`.
/// Resumes an existing session when provided, otherwise starts a new one.
fn init_workflow_session(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
) -> Result<(String, String)> {
    if let Some(sid) = existing_session_id {
        let text = super::handler::workflow(
            context,
            &json!({
                "sessionId": sid,
                "response": "Resuming pre-activated pipeline session"
            }),
        )?;
        return Ok((sid, text));
    }
    let text = super::handler::workflow(context, &json!({ "workflowId": workflow_id }))?;
    let sid = parse_session_id_from_creation(&text).ok_or_else(|| {
        MaenifoldError::Message("Workflow session ID missing from response".to_string())
    })?;
    Ok((sid, text))
}

/// Extracts the session ID from the structured `"Created session: workflow-XXXX"` prefix
/// produced by `start_workflow_session`, replacing the previous regex-based approach.
fn parse_session_id_from_creation(text: &str) -> Option<String> {
    text.strip_prefix("Created session: ")
        .and_then(|rest| rest.split_whitespace().next())
        .filter(|id| id.starts_with("workflow-"))
        .map(str::to_string)
}

/// Returns total step count, preferring the `"Step X/Y"` text already in `step_text`
/// to avoid re-reading and re-parsing the workflow definition file.
fn workflow_total_steps(
    context: &ToolContext,
    workflow_id: &str,
    step_text: &str,
) -> Result<usize> {
    if let Some((_, total)) = parse_step_progress(step_text) {
        return Ok(total as usize);
    }
    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    Ok(workflow_steps(&workflow_value, workflow_id)?.len())
}

/// Determines the current step index using frontmatter `currentStep` with a text fallback.
fn current_step_index(context: &ToolContext, session_id: &str, step: &str) -> usize {
    if let Ok((Some(fm), _, _)) = read_markdown(&workflow_session_uri(session_id), &context.config)
    {
        let index = frontmatter_usize(&fm, "currentStep").saturating_add(1);
        if index > 0 {
            return index;
        }
    }
    if let Some((current, _)) = parse_step_progress(step) {
        let index = current as usize;
        if index > 0 {
            return index;
        }
    }
    1
}

async fn maybe_run_step_thinking(run_ctx: &RunContext<'_>, step: &str) -> Result<()> {
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
    let _ = drive_think_session(
        run_ctx.tool_context,
        run_ctx.agent,
        &think_session_id,
        &topic,
        step,
        &think_opts,
        run_ctx.default_session_opts(),
        run_ctx.pool.as_ref(),
        None,
    )
    .await?;
    Ok(())
}
