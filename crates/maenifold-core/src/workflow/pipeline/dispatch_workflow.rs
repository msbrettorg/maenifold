use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::workflow::{format_workflow_step, read_workflow_definition, workflow_steps};
use serde_json::json;
use serde_yaml::Value as YamlValue;

use super::{PipelineNode, workflow_id_for_node};

/// Activate a workflow node session with a single write that updates its frontmatter
/// status to `active` and renders the first workflow step into the session body.
///
/// Returns `Ok(true)` when activation performed a write, or `Ok(false)` when the
/// session was already active and should be skipped.
pub(super) fn activate_workflow_node_session(
    context: &ToolContext,
    workflow_id: &str,
    session_id: &str,
) -> Result<bool> {
    let workflow_uri = workflow_session_uri(session_id);
    let (session_frontmatter, existing_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut session_frontmatter = session_frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {session_id} has invalid format"
        ))
    })?;

    if session_frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        == Some("active")
    {
        return Ok(false);
    }

    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    let steps = workflow_steps(&workflow_value, workflow_id)?;
    session_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    let updated_content = if let Some(first_step) = steps.first() {
        let heading = format!("Step 1/{}", steps.len());
        let rendered_step = format_workflow_step(first_step)?;
        update_activation_section_frontmatter(
            &mut session_frontmatter,
            &existing_content,
            &heading,
            &rendered_step,
        );
        append_step_section(&existing_content, &heading, &rendered_step)
    } else {
        existing_content
    };

    write_markdown(
        &workflow_uri,
        Some(&session_frontmatter),
        &updated_content,
        &context.config,
    )?;
    Ok(true)
}

pub(super) fn build_workflow_dispatch_response(
    node: &PipelineNode,
    session_id: &str,
) -> Result<serde_json::Value> {
    Ok(json!({
        "nodeId": node.id(),
        "sessionId": session_id,
        "workflowId": workflow_id_for_node(node)?,
    }))
}

fn append_step_section(existing_content: &str, heading: &str, content: &str) -> String {
    let mut new_content = existing_content.to_string();
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("\n## {heading}\n\n{content}\n"));
    new_content
}

fn update_activation_section_frontmatter(
    frontmatter: &mut Frontmatter,
    existing_content: &str,
    heading: &str,
    content: &str,
) {
    let section_summary: String = format!("{heading}: {content}").chars().take(256).collect();
    if existing_content.trim().is_empty() && !frontmatter.contains_key("first") {
        frontmatter.insert(
            "first".to_string(),
            YamlValue::String(section_summary.clone()),
        );
    }
    frontmatter.insert("last".to_string(), YamlValue::String(section_summary));
}
