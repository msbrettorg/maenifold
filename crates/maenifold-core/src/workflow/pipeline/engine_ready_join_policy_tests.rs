use super::{
    EdgeType, JoinPolicy, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    compute_blocked_cascade, compute_ready_set,
};
use std::collections::HashMap;

fn node(id: &str, join_policy: Option<JoinPolicy>) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.into(),
        agent: "agent".into(),
        topic: "t".into(),
        total_thoughts: 1,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        display_label: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        model: None,
        retry_policy: None,
        join_policy,
        input_schema: None,
        output_schema: None,
    }
}

fn state(status: NodeStatus) -> NodeState {
    NodeState {
        session_id: "s".into(),
        status,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: None,
        input_artifact: None,
        output_artifact: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn fan_in_definition(join_policy: Option<JoinPolicy>) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![
            node("a", None),
            node("b", None),
            node("c", None),
            node("terminal", join_policy),
        ],
        edges: ["a", "b", "c"]
            .into_iter()
            .map(|source| PipelineEdge {
                source: source.into(),
                target: "terminal".into(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: None,
            })
            .collect(),
        template_id: None,
        inputs: None,
        input_schema: None,
    }
}

#[test]
fn join_policy_any_fires_with_one_completed_and_two_abandoned() {
    let definition = fan_in_definition(Some(JoinPolicy::Any));
    let states = HashMap::from([
        ("a".to_string(), state(NodeStatus::Completed)),
        ("b".to_string(), state(NodeStatus::Abandoned)),
        ("c".to_string(), state(NodeStatus::Abandoned)),
        ("terminal".to_string(), state(NodeStatus::Pending)),
    ]);

    assert_eq!(
        compute_ready_set(&definition, &states),
        vec!["terminal".to_string()]
    );
}

#[test]
fn join_policy_all_default_still_requires_all_completed() {
    let definition = fan_in_definition(None);
    let states = HashMap::from([
        ("a".to_string(), state(NodeStatus::Completed)),
        ("b".to_string(), state(NodeStatus::Abandoned)),
        ("c".to_string(), state(NodeStatus::Abandoned)),
        ("terminal".to_string(), state(NodeStatus::Pending)),
    ]);

    assert!(compute_ready_set(&definition, &states).is_empty());
}

#[test]
fn join_policy_any_all_abandoned_cascades_to_abandoned() {
    let definition = fan_in_definition(Some(JoinPolicy::Any));
    let states = HashMap::from([
        ("a".to_string(), state(NodeStatus::Abandoned)),
        ("b".to_string(), state(NodeStatus::Abandoned)),
        ("c".to_string(), state(NodeStatus::Abandoned)),
        ("terminal".to_string(), state(NodeStatus::Pending)),
    ]);

    assert_eq!(
        compute_blocked_cascade(&definition, &states),
        vec!["terminal".to_string()]
    );
}

#[test]
fn join_policy_first_fires_with_one_completed_and_two_active() {
    let definition = fan_in_definition(Some(JoinPolicy::First));
    let states = HashMap::from([
        ("a".to_string(), state(NodeStatus::Completed)),
        ("b".to_string(), state(NodeStatus::Active)),
        ("c".to_string(), state(NodeStatus::Active)),
        ("terminal".to_string(), state(NodeStatus::Pending)),
    ]);

    assert_eq!(
        compute_ready_set(&definition, &states),
        vec!["terminal".to_string()]
    );
}

#[test]
fn unspecified_join_policy_deserializes_as_all() {
    let node: PipelineNode = serde_json::from_value(serde_json::json!({
        "id": "terminal",
        "type": "sequentialThinking",
        "topic": "t",
        "agent": "agent"
    }))
    .expect("deserialize node without joinPolicy");

    assert_eq!(node.join_policy(), JoinPolicy::All);
}
