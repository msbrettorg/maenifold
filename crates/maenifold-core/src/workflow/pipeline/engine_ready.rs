pub(super) use super::engine_ready_eval::is_edge_satisfied;
use super::engine_ready_eval::{conditional_only_unreachable_reason, is_terminal_status};
use super::{EdgeType, JoinPolicy, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{Direction::Incoming, prelude::NodeIndex};
use std::collections::HashMap;

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

            let join_policy = node.join_policy();
            !is_node_blocked(&graph, node_index, node_states, &join_policy)
                && incoming_edges_satisfy_ready_rule(&graph, node_index, node_states, &join_policy)
        })
        .map(|node| node.id().to_string())
        .collect()
}

#[cfg(test)]
pub fn compute_unreachable_conditional_set(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    compute_unreachable_conditional_failures(definition, node_states)
        .into_iter()
        .map(|(node_id, _)| node_id)
        .collect()
}

pub(super) fn compute_unreachable_conditional_failures(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<(String, String)> {
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
                    status: NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Retrying,
                    ..
                })
            )
        })
        .filter(|node| {
            let Some(&node_index) = node_indices.get(node.id()) else {
                return false;
            };
            conditional_only_unreachable_reason(&graph, node_index, node_states).is_some()
        })
        .filter_map(|node| {
            let node_index = *node_indices.get(node.id())?;
            let reason = conditional_only_unreachable_reason(&graph, node_index, node_states)?;
            Some((node.id().to_string(), reason))
        })
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

fn is_node_blocked(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
    join_policy: &JoinPolicy,
) -> bool {
    if !matches!(join_policy, JoinPolicy::All) {
        return false;
    }

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
                            Some(NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed)
                        )
                })
        })
}

fn incoming_edges_satisfy_ready_rule(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
    join_policy: &JoinPolicy,
) -> bool {
    match join_policy {
        JoinPolicy::All => incoming_edges_satisfy_all_rule(graph, node_index, node_states),
        JoinPolicy::Any => incoming_edges_satisfy_any_rule(graph, node_index, node_states),
        JoinPolicy::First => incoming_edges_satisfy_first_rule(graph, node_index, node_states),
    }
}
fn incoming_edges_satisfy_all_rule(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    let mut has_incoming = false;
    let mut has_satisfied_edge = false;
    let mut has_non_success_edge = false;
    let mut has_satisfying_non_success_edge = false;

    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            has_incoming = true;
            let satisfied = is_edge_satisfied(edge.weight(), source_id, node_states);
            match edge.weight() {
                EdgeType::Success => {
                    if !satisfied {
                        return false;
                    }
                    has_satisfied_edge = true;
                }
                EdgeType::Always | EdgeType::Conditional { .. } => {
                    has_non_success_edge = true;
                    if satisfied {
                        has_satisfied_edge = true;
                        has_satisfying_non_success_edge = true;
                    }
                }
            }
        }
    }

    if !has_incoming {
        return true;
    }
    if !has_satisfied_edge {
        return false;
    }
    if has_non_success_edge {
        has_satisfying_non_success_edge
    } else {
        true
    }
}
fn incoming_edges_satisfy_any_rule(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    let mut has_success_edge = false;
    let mut has_completed_success = false;
    let mut all_success_sources_terminal = true;
    let mut has_non_success_edge = false;
    let mut has_satisfying_non_success_edge = false;

    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            match edge.weight() {
                EdgeType::Success => {
                    has_success_edge = true;
                    match node_states.get(source_id).map(|state| &state.status) {
                        Some(NodeStatus::Completed) => has_completed_success = true,
                        Some(status) if is_terminal_status(status) => {}
                        _ => all_success_sources_terminal = false,
                    }
                }
                EdgeType::Always | EdgeType::Conditional { .. } => {
                    has_non_success_edge = true;
                    if is_edge_satisfied(edge.weight(), source_id, node_states) {
                        has_satisfying_non_success_edge = true;
                    }
                }
            }
        }
    }

    if !has_success_edge {
        return incoming_edges_satisfy_all_rule(graph, node_index, node_states);
    }
    if !has_completed_success || !all_success_sources_terminal {
        return false;
    }
    !has_non_success_edge || has_satisfying_non_success_edge
}
fn incoming_edges_satisfy_first_rule(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    let mut has_success_edge = false;
    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            if matches!(edge.weight(), EdgeType::Success) {
                has_success_edge = true;
                if is_edge_satisfied(edge.weight(), source_id, node_states) {
                    return true;
                }
            }
        }
    }

    !has_success_edge && incoming_edges_satisfy_all_rule(graph, node_index, node_states)
}
