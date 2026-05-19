use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_yml::Value as YamlValue;

use super::NodeStatus;
use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    node_status_name, persist_pipeline_state, pipeline_definition_from_frontmatter,
    read_pipeline_session, write_workflow_node_session,
};
use super::update_context::inject_predecessor_context_for_node;
use super::view::build_pipeline_view_result;

#[allow(clippy::too_many_lines)]
pub(super) fn restart_pipeline_node(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    if super::lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Cannot restart node '{node_id}' while pipeline is running. Cancel the pipeline first, or use 'rewind' after it stops."
        )));
    }
    let definition = pipeline_definition_from_frontmatter(&frontmatter)?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(YamlValue::as_u64)
        .unwrap_or(3) as u32;
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;

    {
        let state = node_states.get_mut(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;

        if !matches!(
            state.status,
            NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed
        ) {
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} is {} — only abandoned or stuck nodes can be restarted",
                node_status_name(&state.status)
            )));
        }

        if state.attempts >= max_attempts {
            state.completed_at = Some(Utc::now());
            state.status = NodeStatus::Stuck;
            persist_pipeline_state(
                &mut frontmatter,
                &node_states,
                &uri,
                &pipeline_content,
                context,
            )?;
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} has reached max attempts ({max_attempts}) and is now stuck"
            )));
        }

        state.session_id = allocate_node_session_id(context, node_id)?;
        state.status = NodeStatus::Pending;
        state.attempts += 1;
        state.started_at = None;
        state.completed_at = None;
        state.acp_session_id = None;
        state.output = None;
        state.thinking_session_id = None;
        state.tool_provenance = None;
        state.input_tokens = None;
        state.output_tokens = None;
        state.model = None;
        state.failure_reason = None;
        state.retry_after = None;
        state.retry_count = None;
    }

    let state = node_states.get(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;
    write_workflow_node_session(
        context,
        session_id,
        &NodeFile {
            session_id: state.session_id.clone(),
            node_id: node.id().to_string(),
            workflow_id: node.effective_workflow_id(),
        },
    )?;
    inject_predecessor_context_for_node(context, &definition, &node_states, node_id)?;

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        session_id,
        &definition,
        &node_states,
        ambient_prompt,
        None,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
