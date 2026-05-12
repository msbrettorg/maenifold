use std::collections::HashMap;

use chrono::Utc;
use tracing::warn;

use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;

use super::acp_reaper::{ReapResult, reap_acp_sessions};
use super::executor_helpers::load_pipeline_node_states;
use super::session_io::{
    frontmatter_value, persist_pipeline_state_with_status, update_workflow_node_frontmatter,
};
use super::{NodeState, NodeStatus, PipelineDefinition};

pub(super) async fn terminate_node_sessions(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> ReapResult {
    reap_acp_sessions(definition, node_states)
}

#[cfg(test)]
pub(super) fn cancellable_node_session_targets(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<(String, String)> {
    let nodes_by_id = definition
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id().to_string(), node))
        .collect::<HashMap<_, _>>();
    node_states
        .iter()
        .filter_map(|(node_id, state)| {
            let node = nodes_by_id.get(node_id)?;
            if !matches!(
                node,
                super::PipelineNode::Prompt { .. } | super::PipelineNode::SequentialThinking { .. }
            ) {
                return None;
            }
            let session_id = state.acp_session_id.as_deref()?.trim();
            if session_id.is_empty() {
                return None;
            }

            Some((node_id.to_string(), session_id.to_string()))
        })
        .collect()
}

pub fn cancel_active_nodes(context: &ToolContext, session_id: &str) -> crate::error::Result<usize> {
    let active_nodes = load_pipeline_node_states(context, session_id)?
        .into_iter()
        .filter(|(_, state)| matches!(state.status, NodeStatus::Active))
        .map(|(node_id, _)| node_id)
        .collect::<Vec<_>>();

    for node_id in &active_nodes {
        warn!(
            session_id = %session_id,
            node_id = %node_id,
            "cancelling active node per user request"
        );
        super::update::update_pipeline_node_status(
            context,
            session_id,
            node_id,
            "abandoned",
            None,
            None,
            None,
            None,
            Some("pipeline cancelled by user"),
            None,
            None,
            None,
        )?;
    }

    Ok(active_nodes.len())
}

pub fn recover_crashed_nodes(
    context: &ToolContext,
    session_id: &str,
) -> crate::error::Result<usize> {
    let active_nodes = load_pipeline_node_states(context, session_id)?
        .into_iter()
        .filter(|(_, state)| matches!(state.status, NodeStatus::Active))
        .map(|(node_id, _)| node_id)
        .collect::<Vec<_>>();

    for node_id in &active_nodes {
        warn!(
            session_id = %session_id,
            node_id = %node_id,
            "marking crashed node as stuck"
        );
        super::update::update_pipeline_node_status(
            context,
            session_id,
            node_id,
            "stuck",
            None,
            None,
            None,
            None,
            Some("driver crashed — use rewind to retry"),
            None,
            None,
            None,
        )?;
    }

    Ok(active_nodes.len())
}

pub fn finalize_failed_pipeline_execution(context: &ToolContext, session_id: &str) {
    if pipeline_has_non_terminal_nodes(context, session_id) {
        mark_pipeline_stuck(context, session_id);
    }
}

fn pipeline_has_non_terminal_nodes(context: &ToolContext, session_id: &str) -> bool {
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
        return false;
    };
    let Some(frontmatter) = frontmatter else {
        return false;
    };
    let Ok(node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        return false;
    };

    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        )
    })
}

fn mark_pipeline_stuck(context: &ToolContext, session_id: &str) {
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, pipeline_content, _)) = read_markdown(&uri, &context.config) else {
        warn!(
            "[pipeline-driver] failed to read pipeline session {} while marking stuck",
            session_id
        );
        return;
    };
    let Some(mut frontmatter) = frontmatter else {
        warn!(
            "[pipeline-driver] pipeline session {} has invalid frontmatter while marking stuck",
            session_id
        );
        return;
    };
    let Ok(mut node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        warn!(
            "[pipeline-driver] pipeline session {} is missing nodeStates while marking stuck",
            session_id
        );
        return;
    };

    for state in node_states.values_mut() {
        if matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        ) {
            state.completed_at = Some(Utc::now());
            state.status = NodeStatus::Stuck;
            if let Err(err) =
                update_workflow_node_frontmatter(context, &state.session_id, "stuck", None)
            {
                warn!(
                    "[pipeline-driver] failed to update workflow {} for stuck pipeline {}: {}",
                    state.session_id, session_id, err
                );
            }
        }
    }

    if let Err(err) = persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("stuck"),
        &uri,
        &pipeline_content,
        context,
    ) {
        warn!(
            "[pipeline-driver] failed to persist stuck state for {}: {}",
            session_id, err
        );
    }
}
