use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, file_exists, read_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use serde::de::DeserializeOwned;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

use super::{NodeState, NodeStatus};

pub(super) fn read_pipeline_session(
    context: &ToolContext,
    session_id: &str,
) -> Result<(String, Frontmatter, String)> {
    let uri = pipeline_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;

    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} isn't a pipeline session"
        )));
    }

    Ok((uri, frontmatter, content))
}

pub(super) fn derive_pipeline_node_states(
    context: &ToolContext,
    node_states: &HashMap<String, NodeState>,
) -> Result<HashMap<String, NodeState>> {
    let mut derived = node_states.clone();

    for state in derived.values_mut() {
        let workflow_uri = workflow_session_uri(&state.session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            return Err(MaenifoldError::Message(format!(
                "Workflow session {} doesn't exist",
                state.session_id
            )));
        }

        let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
        let frontmatter = frontmatter.ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Workflow session {} has invalid format",
                state.session_id
            ))
        })?;

        if frontmatter.get("type").and_then(YamlValue::as_str) != Some("workflow") {
            return Err(MaenifoldError::Message(format!(
                "Session {} isn't a workflow session",
                state.session_id
            )));
        }

        if let Some(status) = frontmatter
            .get("status")
            .and_then(YamlValue::as_str)
            .and_then(workflow_status_to_node_status)
        {
            let preserves_authoritative_terminal = matches!(
                (&state.status, &status),
                (
                    NodeStatus::Abandoned | NodeStatus::Stuck,
                    NodeStatus::Pending
                        | NodeStatus::Ready
                        | NodeStatus::Active
                        | NodeStatus::Retrying
                )
            );
            if !preserves_authoritative_terminal {
                state.status = status;
            }
        }
    }

    Ok(derived)
}

pub(super) fn workflow_status_to_node_status(status: &str) -> Option<NodeStatus> {
    match status {
        "pending" => Some(NodeStatus::Pending),
        "ready" => Some(NodeStatus::Ready),
        "active" => Some(NodeStatus::Active),
        "completed" => Some(NodeStatus::Completed),
        "abandoned" | "cancelled" => Some(NodeStatus::Abandoned),
        "stuck" => Some(NodeStatus::Stuck),
        "retrying" => Some(NodeStatus::Retrying),
        _ => None,
    }
}

pub(super) fn frontmatter_value<T>(frontmatter: &Frontmatter, key: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let value = frontmatter.get(key).cloned().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing frontmatter '{key}'"
        ))
    })?;
    serde_yaml::from_value(value).map_err(Into::into)
}

pub(super) fn optional_frontmatter_value<T>(
    frontmatter: &Frontmatter,
    key: &str,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    frontmatter
        .get(key)
        .cloned()
        .map(serde_yaml::from_value)
        .transpose()
        .map_err(Into::into)
}
