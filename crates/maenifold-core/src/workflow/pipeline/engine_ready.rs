use super::mapping_resolve::{
    CONDITIONAL_NOT_SATISFIED_REASON, ConditionalPredicateResult, conditional_parse_failure_reason,
    conditional_predicate_result,
};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};
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

            !is_node_blocked(&graph, node_index, node_states)
                && incoming_edges_satisfy_ready_rule(&graph, node_index, node_states)
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

fn incoming_edges_satisfy_ready_rule(
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

fn conditional_only_unreachable_reason(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> Option<String> {
    let mut has_incoming = false;
    let mut has_conditional = false;
    let mut has_non_conditional = false;
    let mut all_sources_terminal = true;
    let mut any_conditional_matched = false;
    let mut parse_failure_reason = None;

    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            has_incoming = true;
            match edge.weight() {
                EdgeType::Conditional { .. } => {
                    has_conditional = true;
                    if !matches!(
                        node_states.get(source_id).map(|state| &state.status),
                        Some(
                            NodeStatus::Completed
                                | NodeStatus::Skipped
                                | NodeStatus::Abandoned
                                | NodeStatus::Stuck,
                        )
                    ) {
                        all_sources_terminal = false;
                    }
                    if let Some(state) = node_states.get(source_id)
                        && state.status == NodeStatus::Completed
                    {
                        match edge.weight() {
                            EdgeType::Conditional { path, equals } => {
                                match conditional_predicate_result(
                                    state.output.as_deref(),
                                    path,
                                    equals,
                                ) {
                                    ConditionalPredicateResult::Satisfied => {
                                        any_conditional_matched = true;
                                    }
                                    ConditionalPredicateResult::Unparseable => {
                                        parse_failure_reason.get_or_insert_with(|| {
                                            conditional_parse_failure_reason(source_id)
                                        });
                                    }
                                    ConditionalPredicateResult::NotSatisfied => {}
                                }
                            }
                            EdgeType::Success | EdgeType::Always => {}
                        }
                    }
                }
                EdgeType::Success | EdgeType::Always => {
                    has_non_conditional = true;
                }
            }
        }
    }

    if has_incoming
        && has_conditional
        && !has_non_conditional
        && all_sources_terminal
        && !any_conditional_matched
    {
        Some(parse_failure_reason.unwrap_or_else(|| CONDITIONAL_NOT_SATISFIED_REASON.to_string()))
    } else {
        None
    }
}

pub(super) fn is_edge_satisfied(
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
        (EdgeType::Conditional { path, equals }, Some(state)) => {
            matches!(state.status, NodeStatus::Completed)
                && conditional_predicate_result(state.output.as_deref(), path, equals)
                    == ConditionalPredicateResult::Satisfied
        }
        (_, None) => false,
    }
}
