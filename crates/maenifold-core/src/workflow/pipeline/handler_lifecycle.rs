use super::NodeStatus;
use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state_with_status,
    read_pipeline_session,
};
use super::supervisor::{
    supervisor_cancel, supervisor_is_running, supervisor_pause, supervisor_resume, supervisor_run,
};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt).map(Some);
    }
    if payload
        .get("pause")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return pause_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("resume")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return resume_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}
fn run_pipeline(context: &ToolContext, session_id: &str, prompt: Option<&str>) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let ambient_prompt = prompt.map(str::trim).filter(|value| !value.is_empty());
    let previous_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .map(str::to_string);

    if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active))
    {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} already has active nodes"
        )));
    }
    if supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    if let Some(prompt) = ambient_prompt {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.to_string()));
    } else {
        frontmatter.remove("prompt");
    }
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("active"),
        &uri,
        &pipeline_content,
        context,
    )?;
    if let Err(error) = supervisor_run(
        context.config.maenifold_root.as_path(),
        session_id,
        ambient_prompt.unwrap_or(""),
    ) {
        persist_pipeline_state_with_status(
            &mut frontmatter,
            &node_states,
            previous_status.as_deref(),
            &uri,
            &pipeline_content,
            context,
        )?;
        return Err(error);
    }

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "status": "started", "sessionId": session_id })),
        OutputFormat::Markdown => {
            serde_json::to_string_pretty(&json!({ "status": "started", "sessionId": session_id }))?
        }
    })
}

fn pause_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if !supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} isn't running"
        )));
    }

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    supervisor_pause(session_id)?;
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("paused"),
        &uri,
        &pipeline_content,
        context,
    )?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "status": "paused", "sessionId": session_id })),
        OutputFormat::Markdown => {
            serde_json::to_string_pretty(&json!({ "status": "paused", "sessionId": session_id }))?
        }
    })
}

fn resume_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let ambient_prompt = frontmatter
        .get("prompt")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let pipeline_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or_default();

    if pipeline_status != "paused" {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is not paused (status: {pipeline_status})"
        )));
    }

    if supervisor_is_running(session_id) {
        supervisor_resume(session_id)?;
    } else {
        supervisor_run(
            context.config.maenifold_root.as_path(),
            session_id,
            ambient_prompt,
        )?;
    }
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("active"),
        &uri,
        &pipeline_content,
        context,
    )?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "status": "resumed", "sessionId": session_id })),
        OutputFormat::Markdown => {
            serde_json::to_string_pretty(&json!({ "status": "resumed", "sessionId": session_id }))?
        }
    })
}

fn cancel_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if !supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} isn't running"
        )));
    }

    supervisor_cancel(session_id)?;
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("cancelled"),
        &uri,
        &pipeline_content,
        context,
    )?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "status": "cancelled", "sessionId": session_id })),
        OutputFormat::Markdown => serde_json::to_string_pretty(
            &json!({ "status": "cancelled", "sessionId": session_id }),
        )?,
    })
}
