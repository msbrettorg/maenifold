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

pub fn compute_pipeline_status(node_states: &HashMap<String, NodeState>) -> &'static str {
    if !node_states.is_empty()
        && node_states
            .values()
            .all(|state| matches!(state.status, NodeStatus::Completed))
    {
        return "completed";
    }

    let has_stuck = node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Stuck));
    let has_active_or_ready = node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active | NodeStatus::Ready));

    if has_stuck && !has_active_or_ready {
        "stuck"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::pipeline::{PipelineEdge, PipelineNode};

    fn pipeline_definition(edges: Vec<PipelineEdge>) -> PipelineDefinition {
        PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "workflow-root".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "middle".to_string(),
                    workflow_id: "workflow-middle".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "tail".to_string(),
                    workflow_id: "workflow-tail".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges,
        }
    }

    fn node_state(status: NodeStatus) -> NodeState {
        NodeState {
            session_id: format!("session-{status:?}"),
            status,
            attempts: 0,
            output: None,
        }
    }

    #[test]
    fn ready_set_marks_root_nodes_ready_when_pending() {
        let definition = pipeline_definition(Vec::new());
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Pending)),
            ("middle".to_string(), node_state(NodeStatus::Active)),
            ("tail".to_string(), node_state(NodeStatus::Completed)),
        ]);

        let ready = compute_ready_set(&definition, &node_states);

        assert_eq!(ready, vec!["root".to_string()]);
    }

    #[test]
    fn ready_set_requires_success_dependencies_completed() {
        let definition = pipeline_definition(vec![PipelineEdge {
            source: "root".to_string(),
            target: "middle".to_string(),
            edge_type: EdgeType::Success,
        }]);

        let blocked_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Active)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        assert!(
            compute_ready_set(&definition, &blocked_states).is_empty(),
            "success dependency must wait for completion"
        );

        let ready_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Completed)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        assert_eq!(
            compute_ready_set(&definition, &ready_states),
            vec!["middle".to_string()]
        );
    }

    #[test]
    fn ready_set_accepts_completed_upstream_for_always_edges() {
        let definition = pipeline_definition(vec![PipelineEdge {
            source: "root".to_string(),
            target: "middle".to_string(),
            edge_type: EdgeType::Always,
        }]);
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Completed)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        let ready = compute_ready_set(&definition, &node_states);

        assert_eq!(ready, vec!["middle".to_string()]);
    }

    #[test]
    fn ready_set_accepts_abandoned_upstream_for_always_edges() {
        let definition = pipeline_definition(vec![PipelineEdge {
            source: "root".to_string(),
            target: "middle".to_string(),
            edge_type: EdgeType::Always,
        }]);
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Abandoned)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        let ready = compute_ready_set(&definition, &node_states);

        assert_eq!(ready, vec!["middle".to_string()]);
    }

    #[test]
    fn ready_set_accepts_stuck_upstream_for_always_edges() {
        let definition = pipeline_definition(vec![PipelineEdge {
            source: "root".to_string(),
            target: "middle".to_string(),
            edge_type: EdgeType::Always,
        }]);
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Stuck)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        let ready = compute_ready_set(&definition, &node_states);

        assert_eq!(ready, vec!["middle".to_string()]);
    }

    #[test]
    fn ready_set_blocks_success_edge_from_abandoned_or_stuck() {
        let definition = pipeline_definition(vec![
            PipelineEdge {
                source: "root".to_string(),
                target: "middle".to_string(),
                edge_type: EdgeType::Success,
            },
            PipelineEdge {
                source: "middle".to_string(),
                target: "tail".to_string(),
                edge_type: EdgeType::Success,
            },
        ]);

        let abandoned_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Abandoned)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Pending)),
        ]);
        let stuck_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Completed)),
            ("middle".to_string(), node_state(NodeStatus::Stuck)),
            ("tail".to_string(), node_state(NodeStatus::Pending)),
        ]);

        assert!(
            compute_ready_set(&definition, &abandoned_states).is_empty(),
            "abandoned upstream on success edge must block"
        );
        assert!(
            compute_ready_set(&definition, &stuck_states).is_empty(),
            "stuck upstream on success edge must block"
        );
    }

    #[test]
    fn ready_set_recomputes_deterministically() {
        let definition = pipeline_definition(vec![PipelineEdge {
            source: "root".to_string(),
            target: "middle".to_string(),
            edge_type: EdgeType::Success,
        }]);
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Completed)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
            ("tail".to_string(), node_state(NodeStatus::Active)),
        ]);

        let first = compute_ready_set(&definition, &node_states);
        let second = compute_ready_set(&definition, &node_states);

        assert_eq!(first, second);
        assert_eq!(first, vec!["middle".to_string()]);
    }

    #[test]
    fn pipeline_status_returns_completed_when_all_nodes_completed() {
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Completed)),
            ("middle".to_string(), node_state(NodeStatus::Completed)),
        ]);

        assert_eq!(compute_pipeline_status(&node_states), "completed");
    }

    #[test]
    fn pipeline_compute_status_empty_node_states_returns_active() {
        let node_states: HashMap<String, NodeState> = HashMap::new();

        assert_eq!(compute_pipeline_status(&node_states), "active");
    }

    #[test]
    fn pipeline_compute_status_all_stuck_returns_stuck() {
        let mut node_states = HashMap::new();
        node_states.insert(
            "n1".to_string(),
            NodeState {
                session_id: "ws-1".to_string(),
                status: NodeStatus::Stuck,
                attempts: 3,
                output: None,
            },
        );
        node_states.insert(
            "n2".to_string(),
            NodeState {
                session_id: "ws-2".to_string(),
                status: NodeStatus::Stuck,
                attempts: 3,
                output: None,
            },
        );

        assert_eq!(compute_pipeline_status(&node_states), "stuck");
    }

    #[test]
    fn pipeline_status_returns_stuck_when_stuck_without_active_or_ready() {
        let node_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Stuck)),
            ("middle".to_string(), node_state(NodeStatus::Pending)),
        ]);

        assert_eq!(compute_pipeline_status(&node_states), "stuck");
    }

    #[test]
    fn pipeline_status_returns_active_when_ready_or_active_nodes_exist() {
        let ready_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Stuck)),
            ("middle".to_string(), node_state(NodeStatus::Ready)),
        ]);
        let active_states = HashMap::from([
            ("root".to_string(), node_state(NodeStatus::Stuck)),
            ("middle".to_string(), node_state(NodeStatus::Active)),
        ]);

        assert_eq!(compute_pipeline_status(&ready_states), "active");
        assert_eq!(compute_pipeline_status(&active_states), "active");
    }
}
