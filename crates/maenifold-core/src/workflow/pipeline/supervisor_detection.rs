use crate::markdown::{get_memory_files, read_markdown};
use crate::sessions::pipeline_session_uri;
use std::collections::HashMap;
use std::path::Path;

use super::session_io::{
    frontmatter_value, persist_pipeline_state_with_status, update_workflow_node_frontmatter,
};
use super::supervisor::{RUNNING_PIPELINES, supervisor_context};
use super::{NodeState, NodeStatus};

#[cfg(test)]
pub(super) fn supervisor_detect_orphans(root: &Path) -> usize {
    supervisor_detect_orphans_excluding(root, None)
}

pub(super) fn supervisor_detect_orphans_excluding(
    root: &Path,
    excluded_session_id: Option<&str>,
) -> usize {
    let context = supervisor_context(root);
    let Ok(session_uris) = get_memory_files(&context.config, Some("memory://pipelines")) else {
        eprintln!("[pipeline-supervisor] failed to list pipeline sessions for orphan detection");
        return 0;
    };

    let running = RUNNING_PIPELINES.lock().clone();
    let orphan_session_ids = session_uris
        .into_iter()
        .filter_map(|uri| {
            let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
                eprintln!(
                    "[pipeline-supervisor] failed to read pipeline session {} during orphan detection",
                    uri
                );
                return None;
            };
            let Some(frontmatter) = frontmatter else {
                eprintln!(
                    "[pipeline-supervisor] pipeline session {} has invalid frontmatter during orphan detection",
                    uri
                );
                return None;
            };
            let Ok(status) = frontmatter_value::<String>(&frontmatter, "status") else {
                eprintln!(
                    "[pipeline-supervisor] pipeline session {} is missing status during orphan detection",
                    uri
                );
                return None;
            };
            if !matches!(status.as_str(), "active" | "paused") {
                return None;
            }

            let session_id = uri.split('/').next_back()?;
            (!running.contains(session_id) && excluded_session_id != Some(session_id))
                .then(|| session_id.to_string())
        })
        .collect::<Vec<_>>();

    for session_id in &orphan_session_ids {
        mark_pipeline_stuck(root, session_id);
    }

    orphan_session_ids.len()
}

pub(super) fn pipeline_has_non_terminal_nodes(root: &Path, session_id: &str) -> bool {
    let context = supervisor_context(root);
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

    node_states
        .values()
        .any(|state| is_non_terminal(&state.status))
}

pub(super) fn mark_pipeline_stuck(root: &Path, session_id: &str) {
    let context = supervisor_context(root);
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, pipeline_content, _)) = read_markdown(&uri, &context.config) else {
        eprintln!(
            "[pipeline-supervisor] failed to read pipeline session {} while marking stuck",
            session_id
        );
        return;
    };
    let Some(mut frontmatter) = frontmatter else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} has invalid frontmatter while marking stuck",
            session_id
        );
        return;
    };
    let Ok(mut node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} is missing nodeStates while marking stuck",
            session_id
        );
        return;
    };

    for state in node_states.values_mut() {
        if is_non_terminal(&state.status) {
            state.status = NodeStatus::Stuck;
            if let Err(err) =
                update_workflow_node_frontmatter(&context, &state.session_id, "stuck", None)
            {
                eprintln!(
                    "[pipeline-supervisor] failed to update workflow {} for stuck pipeline {}: {}",
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
        &context,
    ) {
        eprintln!(
            "[pipeline-supervisor] failed to persist stuck state for {}: {}",
            session_id, err
        );
    }
}

fn is_non_terminal(status: &NodeStatus) -> bool {
    matches!(
        status,
        NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
    )
}
