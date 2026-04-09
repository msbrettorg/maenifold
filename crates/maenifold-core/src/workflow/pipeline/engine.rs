use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{
    Direction::{Incoming, Outgoing},
    prelude::NodeIndex,
    visit::EdgeRef,
};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn compute_ready_set(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    let graph = definition.build_graph();
    let node_indices = graph
        .node_indices()
        .map(|node_index| (graph[node_index].clone(), node_index))
        .collect::<HashMap<_, _>>();

    definition
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node_states.get(node.id()),
                Some(NodeState {
                    status: NodeStatus::Pending,
                    ..
                })
            )
        })
        .filter(|node| {
            let Some(&node_index) = node_indices.get(node.id()) else {
                return false;
            };

            !is_node_blocked(&graph, node_index, node_states)
                && graph
                    .neighbors_directed(node_index, Incoming)
                    .all(|source_index| {
                        let source_id = graph[source_index].as_str();
                        graph
                            .edges_connecting(source_index, node_index)
                            .all(|edge| is_edge_satisfied(edge.weight(), source_id, node_states))
                    })
        })
        .map(|node| node.id().to_string())
        .collect()
}

pub fn has_remaining_work(node_states: &HashMap<String, NodeState>) -> bool {
    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        )
    })
}

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
            if !matches!(edge.weight(), EdgeType::Success) {
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

pub fn compute_pipeline_status(node_states: &HashMap<String, NodeState>) -> &'static str {
    if !node_states.is_empty()
        && node_states
            .values()
            .all(|state| matches!(state.status, NodeStatus::Completed))
    {
        return "completed";
    }

    if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Stuck))
    {
        "stuck"
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Abandoned))
    {
        "abandoned"
    } else {
        "active"
    }
}

fn is_node_blocked(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    graph
        .neighbors_directed(node_index, Incoming)
        .any(|source_index| {
            let source_id = graph[source_index].as_str();
            graph
                .edges_connecting(source_index, node_index)
                .any(|edge| {
                    matches!(edge.weight(), EdgeType::Success)
                        && matches!(
                            node_states.get(source_id).map(|state| &state.status),
                            Some(NodeStatus::Abandoned | NodeStatus::Stuck)
                        )
                })
        })
}

fn is_edge_satisfied(
    edge_type: &EdgeType,
    source_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    match (edge_type, node_states.get(source_id)) {
        (EdgeType::Success, Some(state)) => matches!(state.status, NodeStatus::Completed),
        (EdgeType::Always, Some(state)) => {
            matches!(
                state.status,
                NodeStatus::Completed | NodeStatus::Abandoned | NodeStatus::Stuck
            )
        }
        (_, None) => false,
    }
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
            if !matches!(edge.weight(), EdgeType::Success) {
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
