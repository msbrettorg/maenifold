use std::collections::HashMap;

use serde_yaml::Value as YamlValue;

use crate::error::Result;
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;

use super::mapping_resolve::conditional_predicate_satisfied;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode};

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
    if let (false, Some(instruction)) = (workflow_fallback, node_instruction) {
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
        if edge.target != downstream_node_id {
            return false;
        }
        match &edge.edge_type {
            EdgeType::Success | EdgeType::Always => true,
            EdgeType::Conditional { path, equals } => node_states
                .get(&edge.source)
                .map(|state| {
                    state.status == NodeStatus::Completed
                        && conditional_predicate_satisfied(state.output.as_deref(), path, equals)
                })
                .unwrap_or(false),
        }
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
