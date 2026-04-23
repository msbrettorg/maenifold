use petgraph::Direction::Incoming;
use petgraph::visit::{Bfs, Walker};
use std::collections::{HashMap, HashSet};

use super::super::super::{NodeState, NodeStatus, PredecessorOutput};

pub(super) fn reachable_node_ids(
    graph: &petgraph::stable_graph::StableGraph<String, super::super::super::EdgeType>,
    start: petgraph::prelude::NodeIndex,
) -> HashSet<String> {
    Bfs::new(graph, start)
        .iter(graph)
        .map(|node_index| graph[node_index].clone())
        .collect()
}

pub(super) fn collect_fork_predecessor_context(
    graph: &petgraph::stable_graph::StableGraph<String, super::super::super::EdgeType>,
    reachable: &HashSet<String>,
    source_node_states: &HashMap<String, NodeState>,
) -> HashMap<String, Vec<PredecessorOutput>> {
    let mut result = HashMap::new();

    for node_index in graph.node_indices() {
        let node_id = &graph[node_index];
        if !reachable.contains(node_id.as_str()) {
            continue;
        }
        let has_reachable_predecessor = graph
            .neighbors_directed(node_index, Incoming)
            .any(|pred_index| reachable.contains(graph[pred_index].as_str()));
        if has_reachable_predecessor {
            continue;
        }

        let mut predecessor_outputs = Vec::new();
        for pred_index in graph.neighbors_directed(node_index, Incoming) {
            let pred_id = &graph[pred_index];
            if let Some(state) = source_node_states.get(pred_id.as_str())
                && matches!(state.status, NodeStatus::Completed)
                && let Some(output) = &state.output
            {
                predecessor_outputs.push(PredecessorOutput {
                    node_id: pred_id.clone(),
                    output: output.clone(),
                });
            }
        }
        if !predecessor_outputs.is_empty() {
            predecessor_outputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
            result.insert(node_id.clone(), predecessor_outputs);
        }
    }

    result
}

pub(super) fn format_predecessor_context_content(
    predecessor_outputs: &[PredecessorOutput],
) -> String {
    predecessor_outputs
        .iter()
        .map(|output| format!("### From {}\n\n{}", output.node_id, output.output))
        .collect::<Vec<_>>()
        .join("\n\n")
}
