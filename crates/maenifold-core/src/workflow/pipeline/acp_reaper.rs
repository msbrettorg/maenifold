use std::collections::{HashMap, HashSet};

use serde_json::json;
use tracing::{debug, warn};

use super::{NodeState, PipelineDefinition, PipelineNode};

#[cfg(test)]
static TEST_LIVE_ACP_SESSIONS: std::sync::Mutex<Option<HashSet<String>>> =
    std::sync::Mutex::new(None);
#[cfg(test)]
static TEST_REAPED_ACP_SESSIONS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReapResult {
    pub(super) acp_sessions: usize,
    pub(super) killed_process_groups: usize,
}

pub(super) fn live_acp_sessions() -> HashSet<String> {
    #[cfg(test)]
    if let Some(sessions) = TEST_LIVE_ACP_SESSIONS
        .lock()
        .expect("test live ACP sessions lock")
        .clone()
    {
        return sessions;
    }
    jockey::acp::list_live_sessions_blocking()
        .into_iter()
        .collect()
}

#[cfg(test)]
pub(super) fn set_test_live_acp_sessions(session_ids: &[&str]) {
    *TEST_LIVE_ACP_SESSIONS
        .lock()
        .expect("test live ACP sessions lock") = Some(
        session_ids
            .iter()
            .map(|session_id| (*session_id).to_string())
            .collect(),
    );
    TEST_REAPED_ACP_SESSIONS
        .lock()
        .expect("test reaped ACP sessions lock")
        .clear();
}

#[cfg(test)]
pub(super) fn clear_test_live_acp_sessions() {
    *TEST_LIVE_ACP_SESSIONS
        .lock()
        .expect("test live ACP sessions lock") = None;
    TEST_REAPED_ACP_SESSIONS
        .lock()
        .expect("test reaped ACP sessions lock")
        .clear();
}

#[cfg(test)]
pub(super) fn test_reaped_acp_sessions() -> Vec<String> {
    TEST_REAPED_ACP_SESSIONS
        .lock()
        .expect("test reaped ACP sessions lock")
        .clone()
}

pub(super) fn reaped_json(reaped: ReapResult) -> serde_json::Value {
    json!({
        "acpSessions": reaped.acp_sessions,
        "killedProcessGroups": reaped.killed_process_groups,
    })
}

pub(super) fn orphaned_acp_worker_count(
    node_states: &HashMap<String, NodeState>,
    live_sessions: &HashSet<String>,
) -> usize {
    node_states
        .values()
        .filter_map(|state| state.acp_session_id.as_deref())
        .filter(|session_id| live_sessions.contains(*session_id))
        .count()
}

pub(super) fn acp_reap_targets(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<(String, String)> {
    definition
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node,
                PipelineNode::Prompt { .. } | PipelineNode::SequentialThinking { .. }
            )
        })
        .filter_map(|node| {
            let state = node_states.get(node.id())?;
            let session_id = state.acp_session_id.as_deref()?.trim();
            (!session_id.is_empty()).then(|| (node.id().to_string(), session_id.to_string()))
        })
        .collect()
}

pub(super) fn reap_acp_sessions(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> ReapResult {
    let mut reaped = ReapResult::default();
    for (node_id, session_id) in acp_reap_targets(definition, node_states) {
        #[cfg(test)]
        {
            let mut guard = TEST_LIVE_ACP_SESSIONS
                .lock()
                .expect("test live ACP sessions lock");
            if let Some(sessions) = guard.as_mut() {
                if sessions.remove(&session_id) {
                    TEST_REAPED_ACP_SESSIONS
                        .lock()
                        .expect("test reaped ACP sessions lock")
                        .push(session_id);
                    reaped.acp_sessions += 1;
                }
                continue;
            }
        }
        match jockey::acp::cancel_tracked_session_blocking(&session_id) {
            Ok(outcome) => {
                if outcome.acp_session {
                    reaped.acp_sessions += 1;
                }
                if outcome.killed_process_group {
                    reaped.killed_process_groups += 1;
                }
                debug!(
                    node_id = %node_id,
                    session_id = %session_id,
                    acp_session = outcome.acp_session,
                    killed_process_group = outcome.killed_process_group,
                    "reaped tracked ACP session for pipeline node"
                );
            }
            Err(error) => {
                warn!(
                    node_id = %node_id,
                    session_id = %session_id,
                    error = %error,
                    "failed to reap tracked ACP session for pipeline node"
                );
            }
        }
    }
    reaped
}
