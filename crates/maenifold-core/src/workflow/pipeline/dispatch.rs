use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::{DateTime, Utc};
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

use super::dispatch_non_workflow::activate_non_workflow_node_session;
use super::dispatch_prompt::build_prompt_dispatch_response;
use super::dispatch_thinking::build_thinking_dispatch_response;
use super::dispatch_tool::build_tool_dispatch_response;
use super::dispatch_workflow::{activate_workflow_node_session, build_workflow_dispatch_response};
use crate::markdown::file_exists;
use crate::sessions::workflow_session_uri;
use super::session_io::{
    NodeFile, derive_pipeline_node_states, frontmatter_value, persist_pipeline_state,
    read_pipeline_session, update_workflow_node_frontmatter,
    write_workflow_node_session_with_status,
};
use super::view::node_ids_with_status;
use super::{
    NodeState, NodeStatus, PipelineDefinition, PipelineNode, compute_ready_set,
    workflow_id_for_node,
};

// T-45.7: RTM-337 / RTM-342 / RTM-343 / RTM-344
// Dispatch computes the ready set, atomically claims eligible nodes (pending→active),
// renders the first workflow step into each dispatched session, and returns the result.
#[allow(clippy::too_many_lines)]
pub(super) fn dispatch_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states: HashMap<String, NodeState> = frontmatter_value(&frontmatter, "nodeStates")?;
    release_retrying_nodes(context, session_id, &definition, &mut node_states)?;
    let mut node_states = derive_pipeline_node_states(context, &node_states)?;
    let max_concurrency = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .unwrap_or(8);
    let ready = compute_ready_set(&definition, &node_states);
    let active_count = node_states
        .values()
        .filter(|state| matches!(state.status, NodeStatus::Active))
        .count() as u64;
    let to_dispatch: Vec<String> = ready
        .into_iter()
        .take(max_concurrency.saturating_sub(active_count) as usize)
        .collect();
    let last = frontmatter
        .get("last")
        .and_then(YamlValue::as_u64)
        .unwrap_or(0);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(last + 1)?);

    let mut dispatched = Vec::with_capacity(to_dispatch.len());
    for node_id in &to_dispatch {
        if let Some(dispatched_node) =
            dispatch_node(context, &definition, &mut node_states, node_id)?
        {
            dispatched.push(dispatched_node);
        }
        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;
    }

    // T-93.4: Increment cursor once per completed dispatch cycle.
    // cursor is used for crash-recovery: on restart the driver can read cursor to know
    // how many cycles completed successfully. Nodes already in Active/Completed state
    // are naturally skipped by compute_ready_set, preventing duplicate dispatch.
    let cursor = frontmatter
        .get("cursor")
        .and_then(YamlValue::as_u64)
        .unwrap_or(0);
    frontmatter.insert("cursor".to_string(), serde_yaml::to_value(cursor + 1)?);

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let active_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Active)?;
    let completed_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Completed)?;
    let remaining = node_states
        .values()
        .filter(|state| {
            matches!(
                state.status,
                NodeStatus::Pending | NodeStatus::Active | NodeStatus::Ready | NodeStatus::Retrying
            )
        })
        .count();
    let result = json!({
        "dispatched": dispatched,
        "active": active_nodes,
        "completed": completed_nodes,
        "remaining": remaining,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn release_retrying_nodes(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &mut HashMap<String, NodeState>,
) -> Result<()> {
    let now = Utc::now();
    for (node_id, state) in node_states.iter_mut() {
        if state.status != NodeStatus::Retrying {
            continue;
        }
        ensure_retry_workflow_session(context, session_id, definition, node_id, state)?;
        let Some(retry_after) = state.retry_after.as_deref() else {
            state.status = NodeStatus::Pending;
            state.retry_after = None;
            update_workflow_node_frontmatter(context, &state.session_id, "pending", None)?;
            continue;
        };
        let retry_at = DateTime::parse_from_rfc3339(retry_after)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .inspect_err(|error| {
                tracing::warn!(
                    node_id,
                    session_id = %state.session_id,
                    retry_after,
                    error = %error,
                    "[pipeline] invalid retry_after timestamp, releasing node immediately"
                );
            })
            .ok();
        if retry_at.is_some_and(|retry_at| retry_at > now) {
            continue;
        }
        state.status = NodeStatus::Pending;
        state.retry_after = None;
        update_workflow_node_frontmatter(context, &state.session_id, "pending", None)?;
    }
    Ok(())
}

fn ensure_retry_workflow_session(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_id: &str,
    state: &NodeState,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(&state.session_id);
    if file_exists(&workflow_uri, &context.config)? {
        return Ok(());
    }

    let node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| missing_definition_node(node_id))?;
    write_workflow_node_session_with_status(
        context,
        session_id,
        &NodeFile {
            session_id: state.session_id.clone(),
            node_id: node_id.to_string(),
            workflow_id: node.effective_workflow_id(),
        },
        "retrying",
    )
}

fn dispatch_node(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &mut HashMap<String, NodeState>,
    node_id: &str,
) -> Result<Option<serde_json::Value>> {
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| missing_definition_node(node_id))?;
    ensure_dispatchable(node)?;
    let node_session_id = node_states
        .get(node_id)
        .ok_or_else(|| missing_node_state(node_id))?
        .session_id
        .clone();
    let state = node_states
        .get_mut(node_id)
        .ok_or_else(|| missing_node_state(node_id))?;

    match node {
        PipelineNode::Workflow { .. } => {
            let response = build_workflow_dispatch_response(node, &node_session_id)?;
            let activated = activate_workflow_node_session(
                context,
                workflow_id_for_node(node)?,
                &node_session_id,
            )?;
            state.status = NodeStatus::Active;
            state.started_at = Some(Utc::now());
            state.completed_at = None;
            state.input_tokens = None;
            state.output_tokens = None;
            state.model = None;
            Ok(activated.then_some(response))
        }
        PipelineNode::Prompt { .. } => {
            let response = build_prompt_dispatch_response(node, &node_session_id)?;
            let activated = activate_non_workflow_node_session(context, &node_session_id)?;
            state.status = NodeStatus::Active;
            state.started_at = Some(Utc::now());
            state.completed_at = None;
            state.input_tokens = None;
            state.output_tokens = None;
            state.model = None;
            Ok(activated.then_some(response))
        }
        PipelineNode::SequentialThinking { .. } => {
            let response = build_thinking_dispatch_response(node, &node_session_id)?;
            let activated = activate_non_workflow_node_session(context, &node_session_id)?;
            state.status = NodeStatus::Active;
            state.started_at = Some(Utc::now());
            state.completed_at = None;
            state.input_tokens = None;
            state.output_tokens = None;
            state.model = None;
            Ok(activated.then_some(response))
        }
        PipelineNode::Tool { .. } => {
            let response = build_tool_dispatch_response(node, &node_session_id)?;
            let activated = activate_non_workflow_node_session(context, &node_session_id)?;
            state.status = NodeStatus::Active;
            state.started_at = Some(Utc::now());
            state.completed_at = None;
            state.input_tokens = None;
            state.output_tokens = None;
            state.model = None;
            Ok(activated.then_some(response))
        }
    }
}

fn ensure_dispatchable(node: &PipelineNode) -> Result<()> {
    match node.required_execution_target_field() {
        Some(field) if !node.has_execution_target() => {
            Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{}' cannot dispatch: missing '{field}'",
                node.id()
            )))
        }
        _ => Ok(()),
    }
}

fn missing_node_state(node_id: &str) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "ERROR: Pipeline session missing node state for '{node_id}'"
    ))
}

fn missing_definition_node(node_id: &str) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "ERROR: Pipeline definition missing node '{node_id}'"
    ))
}
