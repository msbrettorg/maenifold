use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{Direction::Incoming, prelude::NodeIndex};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

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
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
        )
    })
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

pub(super) fn has_failed_success_predecessor(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
) -> bool {
    has_failed_success_predecessor_including(definition, node_states, node_id, &HashSet::new())
}

fn has_failed_success_predecessor_including(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
    abandoned: &HashSet<String>,
) -> bool {
    definition
        .edges
        .iter()
        .filter(|edge| edge.target == node_id && matches!(edge.edge_type, EdgeType::Success))
        .any(|edge| {
            abandoned.contains(&edge.source)
                || matches!(
                    node_states.get(&edge.source).map(|state| &state.status),
                    Some(NodeStatus::Abandoned | NodeStatus::Stuck)
                )
        })
}

pub(super) fn collect_abandoned_descendants(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
) -> Vec<String> {
    let mut queue = definition
        .edges
        .iter()
        .filter(|edge| edge.source == node_id && matches!(edge.edge_type, EdgeType::Success))
        .map(|edge| edge.target.clone())
        .collect::<VecDeque<_>>();
    let mut visited = HashSet::new();
    let mut abandoned = BTreeSet::new();
    let mut cascading_failures = HashSet::new();

    while let Some(target_id) = queue.pop_front() {
        if !visited.insert(target_id.clone()) {
            continue;
        }

        if has_failed_success_predecessor_including(
            definition,
            node_states,
            &target_id,
            &cascading_failures,
        ) && matches!(
            node_states.get(&target_id).map(|state| &state.status),
            Some(NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active)
        ) {
            cascading_failures.insert(target_id.clone());
            abandoned.insert(target_id.clone());
        }

        queue.extend(
            definition
                .edges
                .iter()
                .filter(|edge| {
                    edge.source == target_id && matches!(edge.edge_type, EdgeType::Success)
                })
                .map(|edge| edge.target.clone()),
        );
    }

    abandoned.into_iter().collect()
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
