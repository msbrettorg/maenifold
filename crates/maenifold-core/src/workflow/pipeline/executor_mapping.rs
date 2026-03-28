use std::collections::HashMap;

use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::error::Result;
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;

use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) fn resolve_dot_path(value: &JsonValue, path: &str) -> JsonValue {
    let Some(segments) = parse_dot_path(path) else {
        return JsonValue::Null;
    };
    let mut current = value;

    for segment in segments {
        current = match current {
            JsonValue::Object(map) => match map.get(segment.as_str()) {
                Some(value) => value,
                None => return JsonValue::Null,
            },
            JsonValue::Array(items) => match segment
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get(index))
            {
                Some(value) => value,
                None => return JsonValue::Null,
            },
            _ => return JsonValue::Null,
        };
    }

    current.clone()
}

fn parse_dot_path(path: &str) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    let mut chars = path.chars().peekable();
    let mut current = String::new();
    let mut allow_empty_dot = false;

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                    allow_empty_dot = false;
                } else if allow_empty_dot {
                    allow_empty_dot = false;
                } else {
                    return None;
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                let mut index = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == ']' {
                        closed = true;
                        break;
                    }
                    index.push(next);
                }
                if !closed || index.is_empty() {
                    return None;
                }
                segments.push(index);
                allow_empty_dot = true;
            }
            ']' => return None,
            other => {
                current.push(other);
                allow_empty_dot = false;
            }
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    Some(segments)
}

pub(super) fn mapped_tool_args(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    target_node_id: &str,
    static_args: Option<JsonValue>,
) -> Option<JsonValue> {
    let mut merged_args = match static_args.clone() {
        Some(JsonValue::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    let mut applied_mapping = false;

    for edge in definition
        .edges
        .iter()
        .filter(|edge| edge.target == target_node_id)
    {
        let Some(mapping) = edge.mapping.as_ref() else {
            continue;
        };
        let Some(source_state) = node_states.get(&edge.source) else {
            continue;
        };
        if source_state.status != NodeStatus::Completed {
            continue;
        }

        let source_output = source_state
            .output
            .as_deref()
            .and_then(|value| serde_json::from_str::<JsonValue>(value).ok())
            .unwrap_or(JsonValue::Null);
        for (target_key, source_path) in mapping {
            applied_mapping = true;
            merged_args.insert(
                target_key.clone(),
                resolve_dot_path(&source_output, source_path),
            );
        }
    }

    if applied_mapping {
        Some(JsonValue::Object(merged_args))
    } else {
        static_args
    }
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
