use serde_json::{Value, json};
use std::collections::HashMap;

use super::{NodeState, NodeStatus};

pub(super) fn compute_pipeline_runs(node_states: &HashMap<String, NodeState>) -> Vec<Value> {
    let mut runs: Vec<Value> = node_states
        .iter()
        .map(|(node_id, state)| {
            let status_str = match &state.status {
                NodeStatus::Pending => "pending",
                NodeStatus::Ready => "ready",
                NodeStatus::Active => "active",
                NodeStatus::Completed => "completed",
                NodeStatus::Skipped => "skipped",
                NodeStatus::Abandoned => "abandoned",
                NodeStatus::Stuck => "stuck",
                NodeStatus::Retrying => "retrying",
            };
            let outcome: Option<&str> = match &state.status {
                NodeStatus::Completed => Some("succeeded"),
                NodeStatus::Abandoned | NodeStatus::Stuck => Some("failed"),
                NodeStatus::Skipped => Some("skipped"),
                _ => None,
            };
            json!({
                "nodeId": node_id,
                "startedAt": state.started_at.map(|dt| dt.timestamp_millis()),
                "completedAt": state.completed_at.map(|dt| dt.timestamp_millis()),
                "status": status_str,
                "outcome": outcome,
            })
        })
        .collect();
    runs.sort_by(|a, b| {
        a["nodeId"]
            .as_str()
            .unwrap_or("")
            .cmp(b["nodeId"].as_str().unwrap_or(""))
    });
    runs
}
