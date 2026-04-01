use crate::error::Result;
use crate::markdown::{extract_h2_sections, get_memory_files, read_markdown};
use crate::sessions::workflow_session_uri;
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use regex::Regex;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::sync::LazyLock;

use super::executor::{assemble_node_prompt, predecessor_context_content};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

static THOUGHT_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^Thought (\d+)/(\d+) \[([^\]]*)\](?:\s*\(Revises:\s*(\d+)\)|\s*\(Branch:\s*([^)]+)\))?$",
    )
    .expect("thinking trace heading regex")
});

#[allow(clippy::too_many_arguments)]
pub(super) fn assembled_prompt_for_view(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    state: &NodeState,
    ambient_prompt: &str,
) -> Result<Option<String>> {
    if !matches!(state.status, NodeStatus::Active | NodeStatus::Completed) {
        return Ok(None);
    }
    if matches!(node, PipelineNode::Tool { .. }) {
        return Ok(None);
    }

    let predecessor_context =
        match predecessor_context_content(context, definition, node_states, node.id()) {
            Ok(context) => context,
            Err(_) => return Ok(None),
        };
    let has_predecessor_context = predecessor_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    if !has_predecessor_context {
        return Ok(None);
    }

    Ok(Some(assemble_node_prompt(
        context,
        definition,
        node_states,
        node,
        ambient_prompt,
    )?))
}

pub(super) fn parse_workflow_steps(context: &ToolContext, session_id: &str) -> Option<Vec<Value>> {
    let uri = workflow_session_uri(session_id);
    let (frontmatter, content, _checksum) = read_markdown(&uri, &context.config).ok()?;
    let workflow_status = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("status"))
        .and_then(YamlValue::as_str)
        .unwrap_or_default();
    let steps: Vec<(String, String)> = extract_h2_sections(&content)
        .into_iter()
        .filter(|(heading, _)| workflow_step_progress(heading).is_some())
        .collect();
    if steps.is_empty() {
        return None;
    }

    let last_idx = steps.len() - 1;
    Some(
        steps
            .into_iter()
            .enumerate()
            .map(|(index, (heading, body))| {
                let status = if heading.contains('❌') || body.contains('❌') {
                    "failed"
                } else if workflow_status.eq_ignore_ascii_case("completed")
                    || workflow_status.eq_ignore_ascii_case("complete")
                    || heading.contains('✅')
                    || body.contains('✅')
                    || index < last_idx
                {
                    "complete"
                } else {
                    "active"
                };

                json!({
                    "step_name": workflow_step_name(&heading, &body),
                    "status": status,
                    "output": (!body.trim().is_empty()).then_some(body),
                })
            })
            .collect(),
    )
}

pub(super) fn parse_thinking_trace(context: &ToolContext, session_id: &str) -> Option<Vec<Value>> {
    let (_, content, _) = read_markdown(
        &resolve_thinking_session_uri(context, session_id)?,
        &context.config,
    )
    .ok()?;
    let sections = extract_h2_sections(&content);

    Some(
        sections
            .iter()
            .filter(|(heading, _)| !heading.starts_with("Conclusion"))
            .filter_map(|(heading, body)| {
                let caps = THOUGHT_HEADING_RE.captures(heading)?;
                let thought_number: u64 = caps.get(1)?.as_str().parse().ok()?;
                let total_thoughts: u64 = caps.get(2)?.as_str().parse().ok()?;
                let agent = caps.get(3).map(|m| m.as_str().to_string());
                let revises_thought: Option<u64> =
                    caps.get(4).and_then(|m| m.as_str().parse().ok());
                let branch_id = caps.get(5).map(|m| m.as_str().to_string());
                Some(json!({
                    "thought_number": thought_number,
                    "total_thoughts": total_thoughts,
                    "text": body,
                    "is_revision": revises_thought.is_some(),
                    "revises_thought": revises_thought,
                    "branch_id": branch_id,
                    "agent": agent,
                }))
            })
            .collect(),
    )
}

fn resolve_thinking_session_uri(context: &ToolContext, session_id: &str) -> Option<String> {
    let direct_uri = sequential_session_uri(session_id);
    if read_markdown(&direct_uri, &context.config).is_ok() {
        return Some(direct_uri);
    }

    // session_id may be a wrapper workflow session; scan for a child sequential
    // thinking session whose frontmatter parent links back to it.
    find_child_thinking_uri(context, session_id)
}

/// Scans sequential thinking sessions for one whose frontmatter `parent` field
/// links back to the given wrapper workflow session.
fn find_child_thinking_uri(context: &ToolContext, wrapper_session_id: &str) -> Option<String> {
    let expected_parent = format!("[[workflow/{wrapper_session_id}]]");
    let files = get_memory_files(&context.config, Some("memory://thinking/sequential")).ok()?;

    files.into_iter().find(|uri| {
        read_markdown(uri, &context.config)
            .ok()
            .and_then(|(fm, _, _)| fm)
            .and_then(|fm| fm.get("parent")?.as_str().map(str::to_string))
            .is_some_and(|parent| parent == expected_parent)
    })
}

fn workflow_step_progress(heading: &str) -> Option<(u32, u32)> {
    let step = heading.trim().strip_prefix("Step ")?;
    let (current, remainder) = step.split_once('/')?;
    let total = remainder
        .split(|ch: char| ch == ':' || ch.is_whitespace())
        .find(|part| !part.is_empty())?;
    Some((current.parse().ok()?, total.parse().ok()?))
}

fn workflow_step_name(heading: &str, body: &str) -> String {
    heading
        .split_once(':')
        .map(|(_, name)| name.trim())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            serde_json::from_str::<Value>(body).ok().and_then(|step| {
                step.get("name")
                    .and_then(Value::as_str)
                    .or_else(|| step.get("description").and_then(Value::as_str))
                    .or_else(|| step.get("id").and_then(Value::as_str))
                    .map(|value| value.lines().next().unwrap_or(value).trim().to_string())
            })
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| heading.trim().to_string())
}
