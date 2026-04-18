use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{
    Direction::{Incoming, Outgoing},
    prelude::NodeIndex,
    visit::EdgeRef,
};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn compute_blocked_cascade(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    let graph = definition.build_graph();
    let mut blocked_queue = graph
        .node_indices()
        .filter(|&node_index| {
            matches!(
                node_states.get(&graph[node_index]),
                Some(NodeState {
                    status: NodeStatus::Abandoned | NodeStatus::Stuck,
                    ..
                })
            )
        })
        .collect::<VecDeque<_>>();
    let mut cascaded = Vec::new();
    let mut cascaded_set = HashSet::new();

    while let Some(source_index) = blocked_queue.pop_front() {
        for edge in graph.edges_directed(source_index, Outgoing) {
            if !matches!(
                edge.weight(),
                EdgeType::Success | EdgeType::Conditional { .. }
            ) {
                continue;
            }

            let target_index = edge.target();
            let target_id = graph[target_index].clone();
            if cascaded_set.contains(target_id.as_str())
                || !is_cascade_eligible(&graph, target_index, node_states, &cascaded_set)
            {
                continue;
            }

            cascaded_set.insert(target_id.clone());
            cascaded.push(target_id);
            blocked_queue.push_back(target_index);
        }
    }

    cascaded
}

fn is_cascade_eligible(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
    cascaded: &HashSet<String>,
) -> bool {
    let node_id = graph[node_index].as_str();
    if !matches!(
        effective_node_status(node_id, node_states, cascaded),
        Some(NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Retrying)
    ) {
        return false;
    }

    let mut has_incoming_success = false;
    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            if !matches!(
                edge.weight(),
                EdgeType::Success | EdgeType::Conditional { .. }
            ) {
                continue;
            }

            has_incoming_success = true;
            if !matches!(
                effective_node_status(source_id, node_states, cascaded),
                Some(NodeStatus::Abandoned | NodeStatus::Stuck)
            ) {
                return false;
            }
        }
    }

    has_incoming_success
}

fn effective_node_status<'a>(
    node_id: &'a str,
    node_states: &'a HashMap<String, NodeState>,
    cascaded: &HashSet<String>,
) -> Option<NodeStatus> {
    if cascaded.contains(node_id) {
        Some(NodeStatus::Abandoned)
    } else {
        node_states.get(node_id).map(|state| state.status.clone())
    }
}
