use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge};
use std::collections::HashMap;

pub fn compute_ready_set(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    let incoming_edges = build_incoming_edges(definition);

    definition
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node_states.get(&node.id),
                Some(NodeState {
                    status: NodeStatus::Pending,
                    ..
                })
            )
        })
        .filter(|node| {
            let edges = incoming_edges
                .get(node.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            !is_node_blocked(edges, node_states)
                && edges
                    .iter()
                    .all(|edge| is_edge_satisfied(edge, node_states))
        })
        .map(|node| node.id.clone())
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

fn build_incoming_edges<'a>(
    definition: &'a PipelineDefinition,
) -> HashMap<&'a str, Vec<&'a PipelineEdge>> {
    let mut incoming_edges: HashMap<&'a str, Vec<&'a PipelineEdge>> =
        HashMap::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        incoming_edges.entry(node.id.as_str()).or_default();
    }

    for edge in &definition.edges {
        incoming_edges
            .entry(edge.target.as_str())
            .or_default()
            .push(edge);
    }

    incoming_edges
}

fn is_node_blocked(
    incoming_edges: &[&PipelineEdge],
    node_states: &HashMap<String, NodeState>,
) -> bool {
    incoming_edges.iter().any(|edge| {
        matches!(edge.edge_type, EdgeType::Success)
            && matches!(
                node_states.get(&edge.source).map(|state| &state.status),
                Some(NodeStatus::Abandoned | NodeStatus::Stuck)
            )
    })
}

fn is_edge_satisfied(edge: &PipelineEdge, node_states: &HashMap<String, NodeState>) -> bool {
    match (edge.edge_type.clone(), node_states.get(&edge.source)) {
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
    use crate::workflow::pipeline::PipelineNode;

    fn pipeline_definition(edges: Vec<PipelineEdge>) -> PipelineDefinition {
        PipelineDefinition {
            nodes: vec![
                PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "workflow-root".to_string(),
                    acp_client: None,
                },
                PipelineNode {
                    id: "middle".to_string(),
                    workflow_id: "workflow-middle".to_string(),
                    acp_client: None,
                },
                PipelineNode {
                    id: "tail".to_string(),
                    workflow_id: "workflow-tail".to_string(),
                    acp_client: None,
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
    fn pipeline_restart_ready_set_accepts_stuck_upstream_for_always_edges() {
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
