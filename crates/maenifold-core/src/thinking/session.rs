use crate::error::Result;
use crate::markdown::{Frontmatter, file_exists, read_markdown, write_markdown};
use crate::sessions::{
    date_parts_from_session_id, extract_session_timestamp_millis, frontmatter_string,
    now_iso8601_roundtrip, workflow_session_uri,
};
use crate::tools::ToolContext;
use crate::workflow::clear_submachine_frontmatter;
use chrono::Utc;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

use super::render_failure;
use super::validation::SequentialThinkingInput;

pub(crate) fn sequential_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

// T-12.0: RTM NFR-1 - keep SequentialThinking session lifecycle rules intact across module boundaries.
#[allow(clippy::too_many_lines)]
pub(crate) fn validate_session_state(
    context: &ToolContext,
    input: &SequentialThinkingInput,
    uri: &str,
) -> Result<(bool, Option<String>)> {
    if input.session_id_provided && !is_valid_session_id(&input.session_id) {
        let message = format!("Invalid sessionId format: {}", input.session_id);
        return Ok((
            false,
            Some(render_failure(context, "INVALID_SESSION_ID", &message)),
        ));
    }

    if !input.cancel && input.thought_number > 0 && !input.session_id_provided {
        let message = format!(
            "Session {} missing. Start with thoughtNumber=0.",
            input.session_id
        );
        return Ok((
            false,
            Some(render_failure(context, "SESSION_MISSING", &message)),
        ));
    }

    if let Some(parent_workflow_id) = input
        .parent_workflow_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if input.thought_number > 0 {
            let message = "Parent workflow can only be set on first thought";
            return Ok((
                false,
                Some(render_failure(
                    context,
                    "PARENT_WORKFLOW_FIRST_THOUGHT_ONLY",
                    message,
                )),
            ));
        }

        let parent_uri = workflow_session_uri(parent_workflow_id);
        if !file_exists(&parent_uri, &context.config)? {
            let message = format!("Parent workflow '{parent_workflow_id}' not found");
            return Ok((
                false,
                Some(render_failure(
                    context,
                    "PARENT_WORKFLOW_NOT_FOUND",
                    &message,
                )),
            ));
        }

        let (parent_frontmatter, _, _) = read_markdown(&parent_uri, &context.config)?;
        let parent_frontmatter = parent_frontmatter.unwrap_or_default();
        let parent_status = frontmatter_string(&parent_frontmatter, "status");
        let normalized_parent_status = parent_status.trim().to_ascii_lowercase();
        if ["completed", "cancelled", "abandoned"]
            .iter()
            .any(|closed| *closed == normalized_parent_status)
        {
            let message =
                format!("Parent workflow {parent_workflow_id} is {normalized_parent_status}");
            return Ok((
                false,
                Some(render_failure(context, "PARENT_WORKFLOW_CLOSED", &message)),
            ));
        }
    }

    let session_exists = file_exists(uri, &context.config)?;
    if session_exists {
        let (frontmatter, _, _) = read_markdown(uri, &context.config)?;
        let frontmatter = frontmatter.unwrap_or_default();
        let existing_status = frontmatter_string(&frontmatter, "status").to_ascii_lowercase();
        if matches!(
            existing_status.as_str(),
            "completed" | "cancelled" | "abandoned"
        ) {
            let message = format!("Session {} is already {}", input.session_id, existing_status);
            return Ok((
                false,
                Some(render_failure(context, "SESSION_CLOSED", &message)),
            ));
        }
    }

    if input.session_id_provided
        && input.thought_number == 0
        && !session_exists
        && !input.is_revision
    {
        let message = format!(
            "Session {} not found. To start new session, don't provide sessionId.",
            input.session_id
        );
        return Ok((
            false,
            Some(render_failure(context, "SESSION_NOT_FOUND", &message)),
        ));
    }
    if input.thought_number == 0
        && input.branch_id.is_none()
        && session_exists
        && !input.is_revision
        && !input.cancel
    {
        let message = format!(
            "Session {} exists. Use different sessionId or continue existing.",
            input.session_id
        );
        return Ok((
            false,
            Some(render_failure(context, "SESSION_EXISTS", &message)),
        ));
    }
    if input.thought_number > 0 && !session_exists && !input.is_revision && !input.cancel {
        let message = format!(
            "Session {} missing. Start with thoughtNumber=0.",
            input.session_id
        );
        return Ok((
            false,
            Some(render_failure(context, "SESSION_MISSING", &message)),
        ));
    }
    if input.is_revision && !session_exists {
        let message = format!(
            "Cannot revise - session {} doesn't exist.",
            input.session_id
        );
        return Ok((
            false,
            Some(render_failure(context, "SESSION_NOT_FOUND", &message)),
        ));
    }

    Ok((session_exists, None))
}

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
    if let Some(analysis_type) = input
        .analysis_type
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        frontmatter.insert(
            "analysisType".to_string(),
            YamlValue::String(analysis_type.to_string()),
        );
    }
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

pub(crate) fn finalize_session(
    context: &ToolContext,
    uri: &str,
    status: &str,
    thought_number: u64,
) -> Result<()> {
    let (frontmatter, content, _) = read_markdown(uri, &context.config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();
    clear_submachine_frontmatter(&mut frontmatter);
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "thoughtCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(thought_number + 1)),
    );
    if status == "completed" {
        frontmatter.insert(
            "completed".to_string(),
            YamlValue::String(now_iso8601_roundtrip()),
        );
    } else if status == "cancelled" {
        frontmatter.insert(
            "cancelled".to_string(),
            YamlValue::String(now_iso8601_roundtrip()),
        );
    }
    write_markdown(uri, Some(&frontmatter), &content, &context.config)
}

pub(crate) fn is_valid_session_id(session_id: &str) -> bool {
    extract_session_timestamp_millis(session_id).is_some()
}

pub(crate) fn generate_sequential_session_id() -> String {
    let millis = Utc::now().timestamp_millis();
    // T-14.11: RTM FR-16 - derive the suffix from UUID-backed randomness instead of predictable wall-clock timing.
    let entropy = Uuid::new_v4().as_u128() as u32;
    generate_sequential_session_id_with_entropy(millis, entropy)
}

fn generate_sequential_session_id_with_entropy(millis: i64, entropy: u32) -> String {
    let suffix = 10_000 + (entropy % 90_000);
    format!("session-{millis}-{suffix}")
}
