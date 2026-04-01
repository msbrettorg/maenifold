use crate::error::{MaenifoldError, Result};
use crate::markdown::{append_h2_section, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use std::collections::{BTreeSet, HashMap};

use super::executor::predecessor_context_content;
use super::{EdgeType, NodeState, PipelineDefinition};

pub(super) fn inject_predecessor_context(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    completed_node_id: &str,
) -> Result<()> {
    let downstream_node_ids: BTreeSet<&str> = definition
        .edges
        .iter()
        .filter(|edge| {
            edge.source == completed_node_id
                && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
        })
        .map(|edge| edge.target.as_str())
        .collect();

    for downstream_node_id in downstream_node_ids {
        let Some(content) =
            predecessor_context_content(context, definition, node_states, downstream_node_id)?
        else {
            continue;
        };

        let downstream_state = node_states.get(downstream_node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{downstream_node_id}'"
            ))
        })?;
        upsert_h2_section(
            context,
            &workflow_session_uri(&downstream_state.session_id),
            "Predecessor Context",
            &content,
        )?;
    }

    Ok(())
}

pub(super) fn inject_predecessor_context_for_node(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
) -> Result<()> {
    let Some(content) = predecessor_context_content(context, definition, node_states, node_id)?
    else {
        return Ok(());
    };
    let state = node_states.get(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    upsert_h2_section(
        context,
        &workflow_session_uri(&state.session_id),
        "Predecessor Context",
        &content,
    )
}

pub(super) fn replace_h2_section(
    existing_content: &str,
    heading: &str,
    new_content: &str,
) -> Result<String> {
    let mut line_start = 0usize;
    let mut section_start = None;
    let mut section_end = existing_content.len();

    for line in existing_content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            let heading_text = trimmed.trim_end().trim_start_matches('#').trim();
            if section_start.is_some() {
                section_end = line_start;
                break;
            }
            if heading_text == heading {
                section_start = Some(line_start);
            }
        }
        line_start += line.len();
    }

    let section_start = section_start.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Section not found: '{heading}'. Cannot replace a section that doesn't exist."
        ))
    })?;

    let replacement = format!("## {heading}\n\n{}\n", new_content.trim_end());
    Ok(format!(
        "{}{}{}",
        &existing_content[..section_start],
        replacement,
        &existing_content[section_end..]
    ))
}

fn upsert_h2_section(context: &ToolContext, uri: &str, heading: &str, content: &str) -> Result<()> {
    let (frontmatter, existing_content, _) = read_markdown(uri, &context.config)?;
    let frontmatter = frontmatter.unwrap_or_default();

    if has_h2_section(&existing_content, heading) {
        let updated_content = replace_h2_section(&existing_content, heading, content)?;
        write_markdown(uri, Some(&frontmatter), &updated_content, &context.config)
    } else {
        append_h2_section(uri, heading, content, &context.config)
    }
}

fn has_h2_section(existing_content: &str, heading: &str) -> bool {
    existing_content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("## ") && trimmed.trim_start_matches('#').trim() == heading
    })
}
