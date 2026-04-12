use std::sync::Arc;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use agent_client_protocol as acp;
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
    pool: Arc<AgentPool>,
    cwd: std::path::PathBuf,
    model: Option<String>,
    /// ACP session ID for resuming an existing agent session.
    acp_session_id: Option<String>,
}

impl RunContext<'_> {
    fn default_session_opts(&self) -> SessionOpts {
        workflow_driver_session_opts(
            self.cwd.clone(),
            self.model.clone(),
            self.acp_session_id.clone(),
        )
    }
}

fn workflow_driver_session_opts(
    cwd: std::path::PathBuf,
    model: Option<String>,
    acp_session_id: Option<String>,
) -> SessionOpts {
    sanitize_workflow_session_opts(SessionOpts {
        session_id: acp_session_id,
        cwd,
        auto_grant_permissions: true,
        model,
        mode: None,
        thought_level: None,
        config_options: None,
        allowed_tools: None,
        max_turns: None,
        timeout_seconds: None,
        mcp_servers: vec![],
    })
}

pub(crate) fn sanitize_workflow_session_opts(mut opts: SessionOpts) -> SessionOpts {
    let allowed_tools = opts.allowed_tools.take().unwrap_or_else(|| {
        ToolRegistry::global()
            .canonical_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    });
    opts.allowed_tools = Some(
        allowed_tools
            .into_iter()
            .filter(|tool| !is_reentrant_workflow_tool(tool))
            .collect(),
    );
    opts
}

pub(crate) fn is_reentrant_workflow_tool(tool: &str) -> bool {
    matches!(
        tool.to_ascii_lowercase().as_str(),
        "workflow" | "sequentialthinking" | "pipeline" | "acpexec" | "acp_exec"
    )
}

/// Send a single prompt to an ACP agent using a specific pool.
#[allow(dead_code)]
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
    let response_result = match context {
        Some(blocks) => session.prompt_with_context(prompt, blocks).await,
        None => session.prompt(prompt).await,
    };
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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_workflow_with_pool(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    agent: &str,
    prompt: &str,
    cwd: std::path::PathBuf,
    timeout_seconds: Option<u64>,
    model: Option<String>,
    acp_session_id: Option<String>,
    content_blocks: Option<Vec<acp::ContentBlock>>,
    pool: Arc<AgentPool>,
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
        pool,
        cwd,
        model,
        acp_session_id,
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
            if trace_enabled() {
                eprintln!(
                    "[workflow] loop: step_index={} total={} step_preview={:?}",
                    step_index,
                    total_steps,
                    &step[..step.len().min(120)]
                );
            }
            if let Some(status) = terminal_workflow_status(&step) {
                if trace_enabled() {
                    eprintln!("[workflow] terminal status detected: {:?}", status);
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
                if trace_enabled() {
                    eprintln!("[workflow] ERROR in step: {}", &step[..step.len().min(200)]);
                }
                break Err(MaenifoldError::Message(step));
            }

            if trace_enabled() {
                eprintln!("[workflow] step {}/{}", step_index, total_steps);
            }
            maybe_run_step_thinking(&run_ctx, &driver, &step).await?;
            let final_step = step_index >= total_steps;
            if trace_enabled() {
                eprintln!("[workflow] prompting agent (final_step={})", final_step);
            }
            let response =
                prompt_workflow_agent(&driver, prompt, &step, first_context.take()).await?;
            if let Some(value) = workflow_completion_text(&response, final_step) {
                if trace_enabled() {
                    eprintln!("[workflow] conclusion extracted: {} chars", value.len());
                }
                conclusion = Some(value);
            } else if trace_enabled() {
                eprintln!(
                    "[workflow] no conclusion extracted (final_step={})",
                    final_step
                );
            }

            let payload = workflow_step_payload(&run_ctx.session_id, &response, final_step);
            let handler_result = super::handler::workflow(context, &payload)?;
            if trace_enabled() {
                eprintln!(
                    "[workflow] handler returned: {:?}",
                    &handler_result[..handler_result.len().min(200)]
                );
            }
            step = workflow_text(&handler_result);
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

/// Like [`run_workflow_with_pool`] but accepts a fully constructed [`SessionOpts`] directly,
/// forwarding all session configuration (model, mode, thought_level, allowed_tools, etc.)
/// instead of building a stripped-down one internally.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_workflow_with_pool_session_opts(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    agent: &str,
    prompt: &str,
    session_opts: SessionOpts,
    pool: Arc<AgentPool>,
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
        pool,
        cwd: session_opts.cwd.clone(),
        model: session_opts.model.clone(),
        acp_session_id: session_opts.session_id.clone(),
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
        WorkflowRunStatus, append_close_error_context, is_reentrant_workflow_tool,
        sanitize_workflow_session_opts, workflow_driver_session_opts, workflow_failure_result,
    };
    use crate::acp_native::SessionOpts;
    use crate::error::MaenifoldError;
    use std::path::PathBuf;

    fn session_opts(allowed_tools: Option<Vec<&str>>) -> SessionOpts {
        SessionOpts {
            session_id: None,
            cwd: PathBuf::from("."),
            auto_grant_permissions: true,
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
            mcp_servers: vec![],
        }
    }

    #[test]
    fn sanitize_workflow_session_opts_filters_reentrant_tools_from_explicit_allowlist() {
        let sanitized = sanitize_workflow_session_opts(session_opts(Some(vec![
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
    fn sanitize_workflow_session_opts_defaults_to_registry_without_reentrant_tools() {
        let sanitized = sanitize_workflow_session_opts(session_opts(None));
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
    fn workflow_driver_session_opts_preserves_supplied_cwd() {
        let cwd = PathBuf::from("/tmp/workflow-cwd");
        let opts = workflow_driver_session_opts(cwd.clone(), Some("model-x".to_string()), None);

        assert_eq!(opts.cwd, cwd);
        assert_eq!(opts.model.as_deref(), Some("model-x"));
        assert_eq!(opts.session_id, None);
    }

    #[test]
    fn workflow_driver_session_opts_preserves_acp_session_id() {
        let cwd = PathBuf::from("/tmp/workflow-cwd");
        let opts = workflow_driver_session_opts(cwd, None, Some("sess-resume-42".to_string()));

        assert_eq!(opts.session_id.as_deref(), Some("sess-resume-42"));
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
