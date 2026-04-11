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

/// Returns the IDs of terminal nodes — nodes with no outgoing edges in the definition.
pub fn compute_terminal_node_ids(definition: &PipelineDefinition) -> Vec<&str> {
    definition
        .nodes
        .iter()
        .filter(|node| !definition.edges.iter().any(|edge| edge.source == node.id()))
        .map(|node| node.id())
        .collect()
}

/// Status computation that respects terminal-node semantics: a pipeline is
/// `completed` when ALL terminal nodes (leaf nodes with no outgoing edges) are
/// completed, even if upstream nodes were abandoned and bypassed via `always`
/// edges.  Falls through to [`compute_pipeline_status`] for other states.
pub fn compute_pipeline_status_with_definition(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> &'static str {
    let terminal_ids = compute_terminal_node_ids(definition);

    // All terminal nodes completed → pipeline is completed regardless of
    // what happened to non-terminal (upstream) nodes.
    if !terminal_ids.is_empty()
        && terminal_ids.iter().all(|id| {
            matches!(
                node_states.get(*id).map(|s| &s.status),
                Some(NodeStatus::Completed)
            )
        })
    {
        return "completed";
    }

    compute_pipeline_status(node_states)
}

#[cfg(test)]
mod terminal_node_tests {
    use super::*;
    use crate::workflow::pipeline::types::{PipelineEdge, PipelineNode};

    fn make_tool_node(id: &str) -> PipelineNode {
        PipelineNode::Tool {
            id: id.to_string(),
            display_label: None,
            tool_name: "test.tool".to_string(),
            tool_args: None,
            agent: None,
            agent_config: None,
            timeout: None,
            allowed_tools: None,
            max_turns: None,
            model: None,
            thought_level: None,
        }
    }

    fn make_state(id: &str, status: NodeStatus) -> (String, NodeState) {
        (
            id.to_string(),
            NodeState {
                session_id: format!("workflow-{id}"),
                status,
                attempts: 0,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                output: None,
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )
    }

    fn make_edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
        PipelineEdge {
            source: source.to_string(),
            target: target.to_string(),
            edge_type,
            mapping: None,
        }
    }

    /// Scenario from T-124.1: upstream node abandoned, terminal nodes all
    /// completed via `always` edges → pipeline should be `completed`.
    #[test]
    fn terminal_nodes_completed_upstream_abandoned_via_always_edge() {
        // DAG: ms-docs --always--> crta (terminal)
        let definition = PipelineDefinition {
            nodes: vec![make_tool_node("ms-docs"), make_tool_node("crta")],
            edges: vec![make_edge("ms-docs", "crta", EdgeType::Always)],
            inputs: None,
            template_id: None,
        };
        let node_states: HashMap<String, NodeState> = [
            make_state("ms-docs", NodeStatus::Abandoned),
            make_state("crta", NodeStatus::Completed),
        ]
        .into();

        assert_eq!(
            compute_pipeline_status_with_definition(&definition, &node_states),
            "completed",
            "all terminal nodes completed → pipeline completed, even with abandoned upstream"
        );
    }

    /// If any terminal node is NOT completed, fall back to standard logic.
    #[test]
    fn terminal_node_abandoned_returns_abandoned() {
        let definition = PipelineDefinition {
            nodes: vec![make_tool_node("a"), make_tool_node("b")],
            edges: vec![make_edge("a", "b", EdgeType::Always)],
            inputs: None,
            template_id: None,
        };
        let node_states: HashMap<String, NodeState> = [
            make_state("a", NodeStatus::Completed),
            make_state("b", NodeStatus::Abandoned), // terminal but abandoned
        ]
        .into();

        assert_eq!(
            compute_pipeline_status_with_definition(&definition, &node_states),
            "abandoned"
        );
    }

    /// All nodes completed (no abandoned) → still returns `completed`.
    #[test]
    fn all_nodes_completed_returns_completed() {
        let definition = PipelineDefinition {
            nodes: vec![make_tool_node("a"), make_tool_node("b")],
            edges: vec![make_edge("a", "b", EdgeType::Success)],
            inputs: None,
            template_id: None,
        };
        let node_states: HashMap<String, NodeState> = [
            make_state("a", NodeStatus::Completed),
            make_state("b", NodeStatus::Completed),
        ]
        .into();

        assert_eq!(
            compute_pipeline_status_with_definition(&definition, &node_states),
            "completed"
        );
    }

    /// Multiple terminal nodes — all must complete.
    #[test]
    fn multiple_terminal_nodes_all_must_complete() {
        // DAG: root -> synthesis (terminal), root -> deck-gen (terminal)
        let definition = PipelineDefinition {
            nodes: vec![
                make_tool_node("root"),
                make_tool_node("synthesis"),
                make_tool_node("deck-gen"),
            ],
            edges: vec![
                make_edge("root", "synthesis", EdgeType::Success),
                make_edge("root", "deck-gen", EdgeType::Success),
            ],
            inputs: None,
            template_id: None,
        };

        // deck-gen still active → not completed
        let node_states: HashMap<String, NodeState> = [
            make_state("root", NodeStatus::Completed),
            make_state("synthesis", NodeStatus::Completed),
            make_state("deck-gen", NodeStatus::Active),
        ]
        .into();
        assert_eq!(
            compute_pipeline_status_with_definition(&definition, &node_states),
            "active"
        );

        // All terminal nodes done
        let node_states_done: HashMap<String, NodeState> = [
            make_state("root", NodeStatus::Completed),
            make_state("synthesis", NodeStatus::Completed),
            make_state("deck-gen", NodeStatus::Completed),
        ]
        .into();
        assert_eq!(
            compute_pipeline_status_with_definition(&definition, &node_states_done),
            "completed"
        );
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
