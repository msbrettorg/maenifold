use crate::error::{MaenifoldError, Result};
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde_yaml::Value as YamlValue;

pub(super) fn activate_non_workflow_node_session(
    context: &ToolContext,
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

    session_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_markdown(
        &workflow_uri,
        Some(&session_frontmatter),
        &existing_content,
        &context.config,
    )?;
    Ok(true)
}
