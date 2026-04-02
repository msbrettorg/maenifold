use super::context::ToolContext;
use crate::acp_native::{GLOBAL_AGENT_POOL, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
};
use crate::workflow::pipeline::supervisor_runtime;
use crate::workflow::runner::{self, ThinkOptions};
use crate::workflow::think_runner::{DEFAULT_TOTAL_THOUGHTS, WIKILINK_CONTEXT};
use agent_client_protocol as acp;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

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
    let opts = session_opts_from(context.config.maenifold_root.clone(), payload)?;
    let context_blocks = optional_context_blocks(payload)?;
    let prompt_owned = prompt.to_string();
    supervisor_runtime()?.block_on(async move {
        runner::run_prompt_with_pool_context(
            &GLOBAL_AGENT_POOL,
            agent,
            &prompt_owned,
            context_blocks,
            opts,
        )
        .await
    })
}

fn exec_think(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let topic = required_str(payload, "topic")?;
    let total_thoughts = payload
        .get("totalThoughts")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TOTAL_THOUGHTS as u64) as u32;
    let context_blocks = optional_context_blocks(payload)?;
    let opts = session_opts_from(context.config.maenifold_root.clone(), payload)?;
    let think_opts = ThinkOptions {
        total_thoughts,
        model: opts.model.clone(),
        parent_workflow_id: None,
    };

    let topic_owned = topic.to_string();
    let agent_owned = agent.to_string();
    let ctx = ToolContext::new(context.config.clone(), context.output_format);
    supervisor_runtime()?.block_on(async move {
        runner::run_think_with_pool(
            &ctx,
            &agent_owned,
            &topic_owned,
            WIKILINK_CONTEXT,
            think_opts,
            opts,
            Arc::clone(&GLOBAL_AGENT_POOL),
            context_blocks,
        )
        .await
        .map(|result| {
            result
                .conclusion
                .unwrap_or_else(|| "[no conclusion produced]".to_string())
        })
    })
}

fn exec_workflow(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let workflow_id = required_str(payload, "workflowId")?;
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
    let context_blocks = optional_context_blocks(payload)?;
    let opts = session_opts_from(context.config.maenifold_root.clone(), payload)?;

    let workflow_id_owned = workflow_id.to_string();
    let prompt_owned = prompt.to_string();
    let agent_owned = agent.to_string();
    let ctx = ToolContext::new(context.config.clone(), context.output_format);
    supervisor_runtime()?.block_on(async move {
        runner::run_workflow_with_pool(
            &ctx,
            &workflow_id_owned,
            None,
            &agent_owned,
            &prompt_owned,
            opts.timeout_seconds,
            opts.model.clone(),
            context_blocks,
            Arc::clone(&GLOBAL_AGENT_POOL),
        )
        .await
        .map(|result| {
            result
                .conclusion
                .unwrap_or_else(|| "[no conclusion produced]".to_string())
        })
    })
}

pub(crate) fn session_opts_from(cwd: PathBuf, payload: &Value) -> Result<SessionOpts> {
    Ok(SessionOpts {
        cwd,
        auto_grant_permissions: required_bool(payload, "autoGrantPermissions")?,
        model: optional_string(payload, "model"),
        mode: optional_string(payload, "sessionMode"),
        thought_level: optional_string(payload, "thoughtLevel"),
        config_options: payload
            .get("configOptions")
            .and_then(Value::as_object)
            .cloned(),
        allowed_tools: payload
            .get("allowedTools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            }),
        max_turns: payload
            .get("maxTurns")
            .and_then(Value::as_u64)
            .map(|v| v as u32),
        timeout_seconds: payload.get("timeoutSeconds").and_then(Value::as_u64),
        mcp_servers: vec![],
    })
}

fn required_str<'a>(payload: &'a Value, field: &'static str) -> Result<&'a str> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty(field))
}

fn optional_string(payload: &Value, field: &str) -> Option<String> {
    payload.get(field).and_then(Value::as_str).map(String::from)
}

fn required_bool(payload: &Value, field: &'static str) -> Result<bool> {
    payload
        .get(field)
        .and_then(Value::as_bool)
        .ok_or(MaenifoldError::MissingProperty(field))
}

pub(crate) fn optional_context_blocks(payload: &Value) -> Result<Option<Vec<acp::ContentBlock>>> {
    let Some(arr) = payload.get("context").and_then(Value::as_array) else {
        return Ok(None);
    };

    let blocks = arr
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let block = serde_json::from_value::<AcpExecContextBlock>(item.clone())
                .map_err(|e| MaenifoldError::InvalidArgument(format!("context[{i}]: {e}")))?;
            context_block_into_acp(i, block)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(blocks))
}

fn context_block_into_acp(index: usize, block: AcpExecContextBlock) -> Result<acp::ContentBlock> {
    Ok(match block {
        AcpExecContextBlock::Text(AcpExecTextContext { text }) => {
            acp::ContentBlock::Text(acp::TextContent::new(text))
        }
        AcpExecContextBlock::Image(AcpExecImageContext {
            data,
            media_type,
            uri,
        }) => {
            let mut image = acp::ImageContent::new(data, media_type);
            image.uri = uri;
            acp::ContentBlock::Image(image)
        }
        AcpExecContextBlock::Resource(AcpExecResourceContext {
            uri,
            text,
            blob,
            media_type,
        }) => {
            let resource = match (text, blob) {
                (Some(text), None) => {
                    let mut r = acp::TextResourceContents::new(text, uri);
                    r.mime_type = media_type;
                    acp::EmbeddedResourceResource::TextResourceContents(r)
                }
                (None, Some(blob)) => {
                    let mut r = acp::BlobResourceContents::new(blob, uri);
                    r.mime_type = media_type;
                    acp::EmbeddedResourceResource::BlobResourceContents(r)
                }
                (Some(_), Some(_)) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires exactly one of 'text' or 'blob'"
                    )));
                }
                (None, None) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires either 'text' or 'blob'"
                    )));
                }
            };
            acp::ContentBlock::Resource(acp::EmbeddedResource::new(resource))
        }
    })
}
