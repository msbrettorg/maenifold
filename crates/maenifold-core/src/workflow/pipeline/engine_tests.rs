use super::engine::{compute_pipeline_status, compute_ready_set};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use std::collections::HashMap;

fn pipeline_definition(edges: Vec<PipelineEdge>) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "workflow-root".to_string(),
                acp_client: None,
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "middle".to_string(),
                workflow_id: "workflow-middle".to_string(),
                acp_client: None,
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "tail".to_string(),
                workflow_id: "workflow-tail".to_string(),
                acp_client: None,
                prompt: None,

                timeout: None,
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
        thinking_session_id: None,
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
        mapping: None,
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
        mapping: None,
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
        mapping: None,
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
        mapping: None,
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
            mapping: None,
        },
        PipelineEdge {
            source: "middle".to_string(),
            target: "tail".to_string(),
            edge_type: EdgeType::Success,
            mapping: None,
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
        mapping: None,
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
            thinking_session_id: None,
            output: None,
        },
    );
    node_states.insert(
        "n2".to_string(),
        NodeState {
            session_id: "ws-2".to_string(),
            status: NodeStatus::Stuck,
            attempts: 3,
            thinking_session_id: None,
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
