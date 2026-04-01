use super::context::ToolContext;
use crate::acp_native::{SessionOpts, GLOBAL_AGENT_POOL};
use crate::error::{MaenifoldError, Result};
use crate::workflow::pipeline::SUPERVISOR_RUNTIME;
use crate::workflow::runner::{self, ThinkOptions};
use serde_json::Value;

/// Unified ACP invocation tool.  Supports three modes:
///
/// - **prompt** – send a single prompt, return the response text.
/// - **think**  – run a sequential-thinking session, return the conclusion.
/// - **workflow** – run a workflow definition, return the conclusion.
///
/// All modes share the global `AgentPool` for process reuse.
pub fn acp_exec(context: &ToolContext, payload: &Value) -> Result<String> {
    let agent = required_str(payload, "agent")?;
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("prompt");

    match mode {
        "prompt" => exec_prompt(context, payload, agent),
        "think" => exec_think(context, payload, agent),
        "workflow" => exec_workflow(context, payload, agent),
        other => Err(MaenifoldError::InvalidArgument(format!(
            "unknown mode '{other}'; expected prompt, think, or workflow"
        ))),
    }
}

fn exec_prompt(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let prompt = required_str(payload, "prompt")?;
    let opts = session_opts_from(context, payload);
    let prompt_owned = prompt.to_string();
    SUPERVISOR_RUNTIME.block_on(async move {
        runner::run_prompt_with_pool(&GLOBAL_AGENT_POOL, agent, &prompt_owned, opts).await
    })
}

fn exec_think(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let topic = required_str(payload, "topic")?;
    let total_thoughts = payload
        .get("totalThoughts")
        .and_then(Value::as_u64)
        .unwrap_or(5) as u32;
    let model = optional_string(payload, "model");
    let timeout_seconds = payload.get("timeoutSeconds").and_then(Value::as_u64);

    let topic_owned = topic.to_string();
    let agent_owned = agent.to_string();
    let ctx = ToolContext::new(context.config.clone(), context.output_format);
    SUPERVISOR_RUNTIME.block_on(async move {
        runner::run_think(
            &ctx,
            &agent_owned,
            &topic_owned,
            ThinkOptions {
                total_thoughts,
                model,
                timeout_seconds,
                ..ThinkOptions::default()
            },
        )
        .await
        .map(|result| result.conclusion.unwrap_or_default())
    })
}

fn exec_workflow(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let workflow_id = required_str(payload, "workflowId")?;
    let prompt = payload
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("");
    let model = optional_string(payload, "model");

    let workflow_id_owned = workflow_id.to_string();
    let prompt_owned = prompt.to_string();
    let agent_owned = agent.to_string();
    let ctx = ToolContext::new(context.config.clone(), context.output_format);
    SUPERVISOR_RUNTIME.block_on(async move {
        runner::run_workflow(
            &ctx,
            &workflow_id_owned,
            &agent_owned,
            &prompt_owned,
            model,
        )
        .await
        .map(|result| result.conclusion.unwrap_or_default())
    })
}

fn session_opts_from(context: &ToolContext, payload: &Value) -> SessionOpts {
    SessionOpts {
        cwd: context.config.maenifold_root.clone(),
        model: optional_string(payload, "model"),
        mode: optional_string(payload, "mode"),
        thought_level: optional_string(payload, "thoughtLevel"),
        config_options: None,
        allowed_tools: payload
            .get("allowedTools")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect()),
        max_turns: payload.get("maxTurns").and_then(Value::as_u64).map(|v| v as u32),
        timeout_seconds: payload.get("timeoutSeconds").and_then(Value::as_u64),
        mcp_servers: vec![],
    }
}

fn required_str<'a>(payload: &'a Value, field: &'static str) -> Result<&'a str> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty(field))
}

fn optional_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(String::from)
}
