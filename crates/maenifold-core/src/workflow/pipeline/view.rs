use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use std::collections::HashMap;

use super::acp_reaper::{live_acp_sessions, orphaned_acp_worker_count};
use super::lease;
use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, pipeline_definition_from_frontmatter,
    read_pipeline_session,
};
use super::view_mermaid::{edge_type_name, render_pipeline_mermaid};
use super::view_nodes::pipeline_nodes_json_with_live_sessions;
use super::{
    NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status_with_definition,
    compute_ready_set,
};

pub(super) fn view_pipeline_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let result = view_pipeline_session_value(context, session_id)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn view_pipeline_summary_session(
    context: &ToolContext,
    session_id: &str,
) -> Result<String> {
    let result = view_pipeline_summary_session_value(context, session_id)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn view_pipeline_session_value(
    context: &ToolContext,
    session_id: &str,
) -> Result<Value> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition = pipeline_definition_from_frontmatter(&frontmatter)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let ambient_prompt = frontmatter
        .get("prompt")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    // Use the persisted status as the authoritative value.  It was written by
    // the driver after each node transition (via `persist_pipeline_state_with_status`)
    // or explicitly by `force_complete_pipeline`, so it correctly reflects any
    // admin override that would be lost if we re-derived from node states alone.
    let persisted_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .map(str::to_owned);
    let mut result = build_pipeline_view_result(
        context,
        session_id,
        &definition,
        &node_states,
        ambient_prompt,
        persisted_status.as_deref(),
    )?;
    apply_pipeline_metadata(&mut result, &frontmatter);
    Ok(result)
}

pub(super) fn view_pipeline_summary_session_value(
    context: &ToolContext,
    session_id: &str,
) -> Result<Value> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition = pipeline_definition_from_frontmatter(&frontmatter)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let persisted_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .map(str::to_owned);
    let status = persisted_status
        .as_deref()
        .unwrap_or_else(|| compute_pipeline_status_with_definition(&definition, &node_states));
    let nodes = definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_state_for(&node_states, node.id())?;
            Ok(json!({
                "nodeId": node.id(),
                "type": node.node_type_name(),
                "status": state.status,
                "attempts": state.attempts,
                "acpSessionId": state.acp_session_id,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let running = node_states
        .values()
        .filter(|state| matches!(state.status, NodeStatus::Active))
        .count();

    Ok(json!({
        "pipelineId": session_id,
        "status": compact_pipeline_status(status),
        "running": running,
        "nodes": nodes,
    }))
}

pub(super) fn node_ids_with_status(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    expected_status: NodeStatus,
) -> Result<Vec<String>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_state_for(node_states, node.id())?;
            Ok((node.id().to_string(), state.status == expected_status))
        })
        .filter_map(|entry| match entry {
            Ok((node_id, true)) => Some(Ok(node_id)),
            Ok((_, false)) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub(super) fn build_pipeline_view_result(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    ambient_prompt: Option<&str>,
    persisted_status: Option<&str>,
) -> Result<Value> {
    let ready = compute_ready_set(definition, node_states);
    // Prefer an explicitly persisted status (written by the driver or an admin
    // override such as force-complete) over a freshly computed value.  Callers
    // that are constructing the view for freshly-computed, in-memory node states
    // (surgery, restart, status update) pass `None` and get computed status.
    let status = persisted_status
        .unwrap_or_else(|| compute_pipeline_status_with_definition(definition, node_states));
    let active = node_ids_with_status(definition, node_states, NodeStatus::Active)?;
    let completed = node_ids_with_status(definition, node_states, NodeStatus::Completed)?;
    let live_sessions = live_acp_sessions();
    let nodes = pipeline_nodes_json_with_live_sessions(
        context,
        definition,
        node_states,
        ambient_prompt.unwrap_or(""),
        &live_sessions,
    )?;
    let edges = pipeline_edges_json(definition);
    let mermaid = render_pipeline_mermaid(definition, node_states)?;
    let running = lease::session_lock_is_held(session_id, &context.config);
    let final_status = is_pipeline_terminal_status(status);

    let mut result = json!({
        "pipelineId": session_id,
        "status": status,
        "final": final_status,
        "nodes": nodes,
        "edges": edges,
        "ready": ready,
        "active": active,
        "completed": completed,
        "mermaid": mermaid,
        "running": running,
    });
    if let Some(template_id) = definition.template_id.as_deref() {
        result["templateId"] = json!(template_id);
    }
    if let Some(input_schema) = &definition.input_schema {
        result["inputSchema"] = input_schema.clone();
    }
    let orphaned_workers = orphaned_acp_worker_count(node_states, &live_sessions);
    if !running && orphaned_workers > 0 {
        result["degraded"] = json!({
            "reason": "orphaned-acp-workers",
            "count": orphaned_workers,
            "orphanedAcpWorkers": orphaned_workers,
        });
    }

    Ok(result)
}

fn apply_pipeline_metadata(result: &mut Value, frontmatter: &crate::markdown::Frontmatter) {
    if let Some(name) = frontmatter
        .get("name")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        result["name"] = json!(name);
    }
    if let Some(description) = frontmatter
        .get("description")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        result["description"] = json!(description);
    }
}

pub(super) fn is_pipeline_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "abandoned" | "stuck"
    )
}

fn compact_pipeline_status(status: &str) -> &str {
    match status {
        "active" | "retrying" | "running" => "running",
        "completed" => "completed",
        "abandoned" | "cancelled" | "failed" | "stuck" => "abandoned",
        "pending" => "pending",
        _ => "pending",
    }
}

pub(super) fn node_state_for<'a>(
    node_states: &'a HashMap<String, NodeState>,
    node_id: &str,
) -> Result<&'a NodeState> {
    node_states.get(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })
}

fn pipeline_edges_json(definition: &PipelineDefinition) -> Vec<Value> {
    definition
        .edges
        .iter()
        .map(|edge| {
            json!({
                "source": edge.source,
                "target": edge.target,
                "type": edge_type_name(&edge.edge_type),
                "mode": edge.mode,
                "mapping": edge.mapping,
            })
        })
        .collect()
}
