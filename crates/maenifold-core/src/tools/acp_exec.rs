use super::context::ToolContext;
use crate::acp_native::{GLOBAL_AGENT_POOL, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
};
use crate::workflow::runner::{self, ThinkOptions};
use crate::workflow::think_runner::{DEFAULT_TOTAL_THOUGHTS, WIKILINK_CONTEXT};
use agent_client_protocol as acp;
use serde_json::Value;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static ACP_EXEC_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build ACP exec runtime: {error}"))
});

fn acp_exec_runtime() -> crate::error::Result<&'static Runtime> {
    ACP_EXEC_RUNTIME
        .as_ref()
        .map_err(|message| MaenifoldError::Message(message.clone()))
}

/// Unified ACP invocation tool.  Supports three modes:
///
/// - **prompt** – send a single prompt, return the response text.
/// - **think**  – run a sequential-thinking session, return the conclusion.
/// - **workflow** – run a workflow definition, return the conclusion.
///
/// All modes share the global `AgentPool` for process reuse.
pub fn acp_exec(context: &ToolContext, payload: &Value) -> Result<String> {
    let agent = required_str(payload, "agent")?;
    let mode = mode_or_default(payload)?;
    let model = optional_string(payload, "model");
    let started_at = Instant::now();

    let result = match mode {
        "prompt" => exec_prompt(context, payload, agent),
        "think" => exec_think(context, payload, agent),
        "workflow" => exec_workflow(context, payload, agent),
        other => Err(MaenifoldError::InvalidArgument(format!(
            "unknown mode '{other}'; expected prompt, think, or workflow"
        ))),
    };

    log_acp_exec_result(
        agent,
        model.as_deref(),
        mode_for_log(payload),
        started_at,
        &result,
    );

    result
}

fn exec_prompt(context: &ToolContext, payload: &Value, agent: &str) -> Result<String> {
    let prompt = required_str(payload, "prompt")?;
    let opts = session_opts_from(context.config.maenifold_root.clone(), payload)?;
    let context_blocks = optional_context_blocks(payload)?;
    let prompt_owned = prompt.to_string();
    run_acp_future(exec_prompt_async(
        agent,
        &prompt_owned,
        context_blocks,
        opts,
    ))
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
    run_acp_future(exec_think_async(
        ctx,
        agent_owned,
        topic_owned,
        think_opts,
        opts,
        context_blocks,
    ))
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
    run_acp_future(exec_workflow_async(
        ctx,
        workflow_id_owned,
        agent_owned,
        prompt_owned,
        opts,
        context_blocks,
    ))
}

fn run_acp_future<F, T>(future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "AcpExec requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                ));
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => acp_exec_runtime()?.block_on(future),
    }
}

async fn exec_prompt_async(
    agent: &str,
    prompt: &str,
    context_blocks: Option<Vec<acp::ContentBlock>>,
    opts: SessionOpts,
) -> Result<String> {
    runner::run_prompt_with_pool_context(&GLOBAL_AGENT_POOL, agent, prompt, context_blocks, opts)
        .await
}

#[allow(clippy::too_many_arguments)]
async fn exec_think_async(
    ctx: ToolContext,
    agent: String,
    topic: String,
    think_opts: ThinkOptions,
    opts: SessionOpts,
    context_blocks: Option<Vec<acp::ContentBlock>>,
) -> Result<String> {
    runner::run_think_with_pool(
        &ctx,
        &agent,
        &topic,
        WIKILINK_CONTEXT,
        think_opts,
        opts,
        Arc::clone(&GLOBAL_AGENT_POOL),
        context_blocks,
        false,
    )
    .await
    .map(|result| {
        result
            .conclusion
            .unwrap_or_else(|| "[no conclusion produced]".to_string())
    })
}

#[allow(clippy::too_many_arguments)]
async fn exec_workflow_async(
    ctx: ToolContext,
    workflow_id: String,
    agent: String,
    prompt: String,
    opts: SessionOpts,
    context_blocks: Option<Vec<acp::ContentBlock>>,
) -> Result<String> {
    runner::run_workflow_with_pool(
        &ctx,
        &workflow_id,
        None,
        &agent,
        &prompt,
        opts.timeout_seconds,
        opts.model.clone(),
        context_blocks,
        Arc::clone(&GLOBAL_AGENT_POOL),
        false,
    )
    .await
    .and_then(workflow_conclusion_text)
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

fn mode_or_default(payload: &Value) -> Result<&str> {
    match payload.get("mode") {
        Some(value) => value.as_str().ok_or_else(|| {
            MaenifoldError::InvalidArgument(
                "mode must be a string; expected prompt, think, or workflow".to_string(),
            )
        }),
        None => Ok("prompt"),
    }
}

fn workflow_conclusion_text(result: runner::WorkflowResult) -> Result<String> {
    match result.status {
        runner::WorkflowRunStatus::Completed => Ok(result
            .conclusion
            .unwrap_or_else(|| "[no conclusion produced]".to_string())),
        runner::WorkflowRunStatus::Failed(reason) => Err(MaenifoldError::Message(reason)),
        runner::WorkflowRunStatus::Cancelled => Err(MaenifoldError::Message(
            "workflow run was cancelled".to_string(),
        )),
    }
}

fn mode_for_log(payload: &Value) -> &str {
    match payload.get("mode") {
        Some(Value::String(mode)) => mode.as_str(),
        Some(_) => "<invalid>",
        None => "prompt",
    }
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

fn log_acp_exec_result<T>(
    agent: &str,
    model: Option<&str>,
    mode: &str,
    started_at: Instant,
    result: &Result<T>,
) {
    let duration_ms = started_at.elapsed().as_millis();
    match result {
        Ok(_) => tracing::info!(
            agent = %agent,
            model = %model.unwrap_or(""),
            mode = %mode,
            duration_ms = duration_ms,
            "[acp-exec] invocation completed"
        ),
        Err(error) => tracing::error!(
            agent = %agent,
            model = %model.unwrap_or(""),
            mode = %mode,
            duration_ms = duration_ms,
            error = %error,
            "[acp-exec] invocation failed"
        ),
    }
}
