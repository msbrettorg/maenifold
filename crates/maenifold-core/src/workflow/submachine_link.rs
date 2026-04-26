use serde_yml::Value as YamlValue;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, file_exists, read_markdown, write_markdown};
use crate::sessions::frontmatter_string;
use crate::tools::ToolContext;

use super::submachine::{HostSessionType, SubmachineType};
use super::submachine_gate::find_gate_task_status;

pub(super) struct ResolvedSubmachineState {
    pub(super) terminal: bool,
    pub(super) target_description: String,
    pub(super) target_reference: Option<String>,
}

pub(super) fn push_related_link(frontmatter: &mut Frontmatter, related_link: &str) {
    let mut related = match frontmatter.remove("related") {
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
        related.push(YamlValue::String(related_link.to_string()));
    }
    frontmatter.insert("related".to_string(), YamlValue::Sequence(related));
}

pub(super) fn validate_submachine_registration(
    context: &ToolContext,
    host_type: HostSessionType,
    host_session_id: &str,
    submachine_type: SubmachineType,
    submachine_session_id: &str,
) -> Result<()> {
    let Some(child_kind) = submachine_type.as_session_kind() else {
        return Ok(());
    };

    if child_kind == host_type && submachine_session_id == host_session_id {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{} cannot register itself as a {} submachine",
            host_type.display_name(),
            submachine_type.as_str()
        )));
    }

    let child_link = child_kind.link(submachine_session_id);
    let host_link = host_type.link(host_session_id);
    if session_ancestry_contains(context, host_type, host_session_id, &child_link)? {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Registering {} '{}' under {} '{}' would create a cycle",
            submachine_type.as_str(),
            submachine_session_id,
            host_type.as_str(),
            host_session_id
        )));
    }

    if let Some(existing_parent) = session_parent_link(context, child_kind, submachine_session_id)?
    {
        if existing_parent != host_link {
            return Err(MaenifoldError::InvalidArgument(format!(
                "{} '{}' is already linked to parent {}",
                submachine_type.as_str(),
                submachine_session_id,
                existing_parent
            )));
        }
    }

    Ok(())
}

pub(super) fn link_child_to_parent(
    context: &ToolContext,
    child_kind: HostSessionType,
    child_session_id: &str,
    host_type: HostSessionType,
    host_session_id: &str,
) -> Result<()> {
    let child_uri = child_kind.session_uri(child_session_id);
    let (child_frontmatter, child_content, _) = read_markdown(&child_uri, &context.config)?;
    let mut child_frontmatter = child_frontmatter.unwrap_or_default();
    let expected_parent = host_type.link(host_session_id);
    if frontmatter_string(&child_frontmatter, "parent").is_empty() {
        child_frontmatter.insert("parent".to_string(), YamlValue::String(expected_parent));
        write_markdown(
            &child_uri,
            Some(&child_frontmatter),
            &child_content,
            &context.config,
        )?;
    }
    Ok(())
}

fn session_parent_link(
    context: &ToolContext,
    session_type: HostSessionType,
    session_id: &str,
) -> Result<Option<String>> {
    let uri = session_type.session_uri(session_id);
    let (frontmatter, _, _) = read_markdown(&uri, &context.config)?;
    let parent = frontmatter
        .as_ref()
        .map(|values| frontmatter_string(values, "parent"))
        .filter(|value| !value.is_empty());
    Ok(parent)
}

fn session_ancestry_contains(
    context: &ToolContext,
    session_type: HostSessionType,
    session_id: &str,
    expected_ancestor_link: &str,
) -> Result<bool> {
    let mut current = session_parent_link(context, session_type, session_id)?;
    while let Some(parent_link) = current {
        if parent_link == expected_ancestor_link {
            return Ok(true);
        }
        let Some((parent_type, parent_session_id)) = parse_session_link(&parent_link) else {
            return Ok(false);
        };
        current = session_parent_link(context, parent_type, &parent_session_id)?;
    }
    Ok(false)
}

fn parse_session_link(value: &str) -> Option<(HostSessionType, String)> {
    let normalized = value.trim().strip_prefix("[[")?.strip_suffix("]]")?;
    let (session_type, session_id) = normalized.split_once('/')?;
    let session_type = match session_type {
        "workflow" => HostSessionType::Workflow,
        "sequential" => HostSessionType::Sequential,
        _ => return None,
    };
    Some((session_type, session_id.to_string()))
}

pub(super) fn resolved_terminal_state(
    context: &ToolContext,
    submachine_type: SubmachineType,
    submachine_session_id: &str,
    persisted_target: Option<&str>,
) -> Result<Option<ResolvedSubmachineState>> {
    match submachine_type {
        SubmachineType::Sequential | SubmachineType::Workflow => {
            let session_uri = submachine_type
                .as_session_kind()
                .expect("non-gate session kinds are supported")
                .session_uri(submachine_session_id);
            if !file_exists(&session_uri, &context.config)? {
                return Ok(None);
            }
            let (frontmatter, _, _) = read_markdown(&session_uri, &context.config)?;
            let status = frontmatter
                .as_ref()
                .map(|values| frontmatter_string(values, "status"))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "active".to_string());
            let terminal = matches!(status.as_str(), "completed" | "cancelled" | "abandoned");
            Ok(Some(ResolvedSubmachineState {
                terminal,
                target_description: status,
                target_reference: Some(session_uri),
            }))
        }
        SubmachineType::Gate => Ok(find_gate_task_status(
            context,
            submachine_session_id,
            persisted_target,
        )?
        .map(|task| ResolvedSubmachineState {
            terminal: task.terminal,
            target_description: task.effective_state,
            target_reference: Some(task.path.to_string_lossy().into_owned()),
        })),
    }
}
