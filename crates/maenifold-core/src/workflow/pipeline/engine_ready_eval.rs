use super::mapping_resolve::{
    CONDITIONAL_NOT_SATISFIED_REASON, ConditionalPredicateResult, conditional_parse_failure_reason,
    conditional_predicate_result,
};
use super::{EdgeType, NodeState, NodeStatus};
use petgraph::{Direction::Incoming, prelude::NodeIndex};
use std::collections::HashMap;

pub(super) fn conditional_only_unreachable_reason(
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

pub(super) fn is_terminal_status(status: &NodeStatus) -> bool {
    matches!(
        status,
        NodeStatus::Completed | NodeStatus::Skipped | NodeStatus::Abandoned | NodeStatus::Stuck
    )
}
