use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::markdown::file_exists;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;

use super::super::session_io::{
    NodeFile, update_workflow_node_frontmatter, write_workflow_node_session_with_status,
};
use super::super::{NodeState, NodeStatus, PipelineDefinition};

pub(super) fn release_retrying_nodes(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &mut HashMap<String, NodeState>,
) -> Result<()> {
    let now = Utc::now();
    for (node_id, state) in node_states.iter_mut() {
        if state.status != NodeStatus::Retrying {
            continue;
        }
        ensure_retry_workflow_session(context, session_id, definition, node_id, state)?;
        let Some(retry_after) = state.retry_after.as_deref() else {
            state.status = NodeStatus::Pending;
            state.retry_after = None;
            update_workflow_node_frontmatter(context, &state.session_id, "pending", None)?;
            continue;
        };
        let retry_at = DateTime::parse_from_rfc3339(retry_after)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .inspect_err(|error| {
                tracing::warn!(
                    node_id,
                    session_id = %state.session_id,
                    retry_after,
                    error = %error,
                    "[pipeline] invalid retry_after timestamp, releasing node immediately"
                );
            })
            .ok();
        if retry_at.is_some_and(|retry_at| retry_at > now) {
            continue;
        }
        state.status = NodeStatus::Pending;
        state.retry_after = None;
        update_workflow_node_frontmatter(context, &state.session_id, "pending", None)?;
    }
    Ok(())
}

fn ensure_retry_workflow_session(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_id: &str,
    state: &NodeState,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(&state.session_id);
    if file_exists(&workflow_uri, &context.config)? {
        return Ok(());
    }

    let node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| super::missing_definition_node(node_id))?;
    write_workflow_node_session_with_status(
        context,
        session_id,
        &NodeFile {
            session_id: state.session_id.clone(),
            node_id: node_id.to_string(),
            workflow_id: node.effective_workflow_id(),
        },
        "retrying",
    )
}
