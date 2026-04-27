//! T-01.17: session initialization helpers split from [`super::session`]
//! to keep the parent file under the per-file LOC budget. No behavioral changes.

use super::validation::SequentialThinkingInput;
use crate::error::Result;
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::collections::BTreeMap;

pub(crate) fn initialize_session(
    context: &ToolContext,
    input: &SequentialThinkingInput,
    uri: &str,
) -> Result<()> {
    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Sequential Thinking Session {}", input.session_id)),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(input.session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
    if let Some(parent_workflow_id) = input
        .parent_workflow_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        frontmatter.insert(
            "parent".to_string(),
            YamlValue::String(format!("[[workflow/{parent_workflow_id}]]")),
        );
    }
    write_markdown(
        uri,
        Some(&frontmatter),
        "# Sequential Thinking Session\n\n",
        &context.config,
    )?;

    if let Some(parent_workflow_id) = input
        .parent_workflow_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        link_parent_workflow(context, parent_workflow_id, &input.session_id)?;
    }

    Ok(())
}

fn link_parent_workflow(
    context: &ToolContext,
    parent_workflow_id: &str,
    session_id: &str,
) -> Result<()> {
    let parent_uri = workflow_session_uri(parent_workflow_id);
    let (parent_frontmatter, parent_content, _) = read_markdown(&parent_uri, &context.config)?;
    let mut parent_frontmatter = parent_frontmatter.unwrap_or_default();
    let related_link = format!("[[sequential/{session_id}]]");
    let mut related = match parent_frontmatter.remove("related") {
        Some(YamlValue::Sequence(values)) => values,
        Some(YamlValue::String(value)) if !value.trim().is_empty() => {
            vec![YamlValue::String(value)]
        }
        _ => Vec::new(),
    };
    if !related
        .iter()
        .filter_map(YamlValue::as_str)
        .any(|existing| existing == related_link)
    {
        related.push(YamlValue::String(related_link));
    }
    parent_frontmatter.insert("related".to_string(), YamlValue::Sequence(related));
    write_markdown(
        &parent_uri,
        Some(&parent_frontmatter),
        &parent_content,
        &context.config,
    )?;
    Ok(())
}
