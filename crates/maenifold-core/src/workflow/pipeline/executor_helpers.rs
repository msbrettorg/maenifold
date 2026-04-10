use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::Value as YamlValue;

use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;

use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state_with_status,
    read_pipeline_session, update_workflow_node_frontmatter,
};
use super::update::update_pipeline_node_status;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode,
    compute_pipeline_status_with_definition,
};

pub(super) const DEFAULT_MAX_CONCURRENCY: usize = 8;
pub(super) const MAX_CONCLUSION_CHARS: usize = 4096;

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    data: T,
    error: Option<EnvelopeError>,
}

#[derive(Debug, Deserialize)]
struct EnvelopeError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchResult {
    dispatched: Vec<DispatchedNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DispatchedNode {
    pub(super) node_id: String,
    pub(super) session_id: Option<String>,
}

pub(super) fn load_pipeline_node_states(
    context: &ToolContext,
    session_id: &str,
) -> Result<HashMap<String, NodeState>> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)
}

pub(super) fn load_execution_config(
    context: &ToolContext,
    session_id: &str,
) -> Result<(PipelineDefinition, usize)> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let max_concurrency = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_CONCURRENCY);
    Ok((definition, max_concurrency))
}

pub(super) fn parse_dispatch_result(raw: String) -> Result<Vec<DispatchedNode>> {
    let response: Envelope<DispatchResult> = serde_json::from_str(&raw)?;
    if response.success {
        Ok(response.data.dispatched)
    } else {
        Err(MaenifoldError::Message(
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "ERROR: Pipeline dispatch failed".to_string()),
        ))
    }
}

pub fn abandon_active_nodes(context: &ToolContext, session_id: &str) -> Result<usize> {
    let active_nodes = load_pipeline_node_states(context, session_id)?
        .into_iter()
        .filter(|(_, state)| matches!(state.status, NodeStatus::Active))
        .map(|(node_id, _)| node_id)
        .collect::<Vec<_>>();

    for node_id in &active_nodes {
        update_pipeline_node_status(context, session_id, node_id, "abandoned", None, None, None)?;
    }

    Ok(active_nodes.len())
}

pub fn finalize_failed_pipeline_execution(context: &ToolContext, session_id: &str) {
    if pipeline_has_non_terminal_nodes(context, session_id) {
        mark_pipeline_stuck(context, session_id);
    }
}

fn pipeline_has_non_terminal_nodes(context: &ToolContext, session_id: &str) -> bool {
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
        return false;
    };
    let Some(frontmatter) = frontmatter else {
        return false;
    };
    let Ok(node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        return false;
    };

    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        )
    })
}

fn mark_pipeline_stuck(context: &ToolContext, session_id: &str) {
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, pipeline_content, _)) = read_markdown(&uri, &context.config) else {
        eprintln!(
            "[pipeline-driver] failed to read pipeline session {} while marking stuck",
            session_id
        );
        return;
    };
    let Some(mut frontmatter) = frontmatter else {
        eprintln!(
            "[pipeline-driver] pipeline session {} has invalid frontmatter while marking stuck",
            session_id
        );
        return;
    };
    let Ok(mut node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        eprintln!(
            "[pipeline-driver] pipeline session {} is missing nodeStates while marking stuck",
            session_id
        );
        return;
    };

    for state in node_states.values_mut() {
        if matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        ) {
            state.status = NodeStatus::Stuck;
            if let Err(err) =
                update_workflow_node_frontmatter(context, &state.session_id, "stuck", None)
            {
                eprintln!(
                    "[pipeline-driver] failed to update workflow {} for stuck pipeline {}: {}",
                    state.session_id, session_id, err
                );
            }
        }
    }

    if let Err(err) = persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("stuck"),
        &uri,
        &pipeline_content,
        context,
    ) {
        eprintln!(
            "[pipeline-driver] failed to persist stuck state for {}: {}",
            session_id, err
        );
    }
}

pub(super) fn derive_execution_status(
    context: &ToolContext,
    session_id: &str,
    cancelled: bool,
) -> Result<String> {
    if cancelled {
        return Ok("cancelled".to_string());
    }

    let node_states = load_pipeline_node_states(context, session_id)?;

    // Use terminal-node-aware status when the definition is available.
    if let Ok((definition, _)) = load_execution_config(context, session_id) {
        return Ok(
            compute_pipeline_status_with_definition(&definition, &node_states).to_string(),
        );
    }

    // Fallback (definition unreadable): derive from raw node states.
    if node_states
        .values()
        .all(|state| matches!(state.status, NodeStatus::Completed))
    {
        Ok("completed".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Stuck))
    {
        Ok("stuck".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Abandoned))
    {
        Ok("abandoned".to_string())
    } else {
        Ok("active".to_string())
    }
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

pub(super) fn assemble_node_prompt(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    ambient_prompt: &str,
) -> Result<String> {
    let predecessor_context =
        predecessor_context_content(context, definition, node_states, node.id())?;
    let ambient_prompt = ambient_prompt.trim();
    let node_instruction = match node {
        PipelineNode::Workflow { prompt, .. } => prompt.as_deref(),
        PipelineNode::Prompt { prompt, .. } => Some(prompt.as_str()),
        PipelineNode::SequentialThinking { topic, .. } => Some(topic.as_str()),
        PipelineNode::Tool { .. } => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty());
    let workflow_fallback =
        matches!(node, PipelineNode::Workflow { .. }) && node_instruction.is_none();
    let mut parts = Vec::new();

    if let Some(context) = predecessor_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(context.to_string());
    }
    if !ambient_prompt.is_empty() {
        parts.push(ambient_prompt.to_string());
    }
    if !workflow_fallback && let Some(instruction) = node_instruction {
        parts.push(instruction.to_string());
    }

    Ok(parts.join("\n\n"))
}

pub(super) fn predecessor_context_content(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    downstream_node_id: &str,
) -> Result<Option<String>> {
    let mut sections = Vec::new();

    for edge in definition.edges.iter().filter(|edge| {
        edge.target == downstream_node_id
            && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
    }) {
        let Some(upstream_state) = node_states.get(&edge.source) else {
            continue;
        };
        if upstream_state.status != NodeStatus::Completed {
            continue;
        }
        let Some(content) =
            predecessor_context_source(context, definition, &edge.source, upstream_state)?
        else {
            continue;
        };
        sections.push(format!("### From {}\n\n{}", edge.source, content));
    }

    Ok((!sections.is_empty()).then(|| sections.join("\n\n")))
}

fn predecessor_context_source(
    context: &ToolContext,
    definition: &PipelineDefinition,
    upstream_node_id: &str,
    upstream_state: &NodeState,
) -> Result<Option<String>> {
    if let Some(output) = upstream_state
        .output
        .as_deref()
        .map(str::trim)
        .filter(|output| !output.is_empty())
    {
        return Ok(Some(output.to_string()));
    }

    let Some(upstream_node) = definition
        .nodes
        .iter()
        .find(|node| node.id() == upstream_node_id)
    else {
        return Ok(None);
    };
    if upstream_node.workflow_id().is_none() {
        return Ok(None);
    }

    let (frontmatter, _, _) = read_markdown(
        &workflow_session_uri(&upstream_state.session_id),
        &context.config,
    )?;
    Ok(frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("last"))
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|last| !last.is_empty())
        .map(ToOwned::to_owned))
}
