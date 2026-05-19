use super::engine_ready::{
    compute_ready_set, compute_unreachable_conditional_set, is_edge_satisfied,
};
use super::types::{EdgeType, PipelineEdge, PipelineNode};
use super::{NodeState, NodeStatus, PipelineDefinition};
use std::collections::HashMap;

fn completed_state(output: &str) -> NodeState {
    NodeState {
        session_id: "s".into(),
        status: NodeStatus::Completed,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: Some(output.into()),
        input_artifact: None,
        output_artifact: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn pending_state() -> NodeState {
    NodeState {
        session_id: "s".into(),
        status: NodeStatus::Pending,
        attempts: 0,
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

fn make_thinking_node(id: &str) -> PipelineNode {
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
        join_policy: None,
        input_schema: None,
        output_schema: None,
    }
}

#[test]
fn conditional_edge_satisfied_when_path_matches() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"verdict":"FAIL"}"#));
    let edge = EdgeType::Conditional {
        path: "verdict".into(),
        equals: serde_json::Value::String("FAIL".into()),
    };
    assert!(is_edge_satisfied(&edge, "up", &states));
}

#[test]
fn conditional_edge_not_satisfied_when_value_differs() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
    let edge = EdgeType::Conditional {
        path: "verdict".into(),
        equals: serde_json::Value::String("FAIL".into()),
    };
    assert!(!is_edge_satisfied(&edge, "up", &states));
}

#[test]
fn conditional_edge_not_satisfied_when_path_missing() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"other":"FAIL"}"#));
    let edge = EdgeType::Conditional {
        path: "verdict".into(),
        equals: serde_json::Value::String("FAIL".into()),
    };
    assert!(!is_edge_satisfied(&edge, "up", &states));
}

#[test]
fn conditional_edge_not_satisfied_when_output_not_json() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state("plain text"));
    let edge = EdgeType::Conditional {
        path: "verdict".into(),
        equals: serde_json::Value::String("FAIL".into()),
    };
    assert!(!is_edge_satisfied(&edge, "up", &states));
}

#[test]
fn conditional_edge_supports_number_and_bool_equality() {
    let mut states = HashMap::new();
    states.insert(
        "up".to_string(),
        completed_state(r#"{"score":42,"ok":true}"#),
    );
    let num_edge = EdgeType::Conditional {
        path: "score".into(),
        equals: serde_json::json!(42),
    };
    let bool_edge = EdgeType::Conditional {
        path: "ok".into(),
        equals: serde_json::json!(true),
    };
    assert!(is_edge_satisfied(&num_edge, "up", &states));
    assert!(is_edge_satisfied(&bool_edge, "up", &states));
}

#[test]
fn conditional_only_target_with_unmet_predicate_is_not_ready() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
    states.insert("down".to_string(), pending_state());
    let definition = PipelineDefinition {
        nodes: vec![make_thinking_node("up"), make_thinking_node("down")],
        edges: vec![PipelineEdge {
            source: "up".into(),
            target: "down".into(),
            edge_type: EdgeType::Conditional {
                path: "verdict".into(),
                equals: serde_json::Value::String("FAIL".into()),
            },
            mode: Default::default(),
            mapping: None,
        }],
        template_id: None,
        inputs: None,
        input_schema: None,
    };
    let ready = compute_ready_set(&definition, &states);
    assert!(
        ready.is_empty(),
        "expected no ready nodes, got: {:?}",
        ready
    );
}

#[test]
fn conditional_target_becomes_ready_when_predicate_matches() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"verdict":"FAIL"}"#));
    states.insert("down".to_string(), pending_state());
    let definition = PipelineDefinition {
        nodes: vec![make_thinking_node("up"), make_thinking_node("down")],
        edges: vec![PipelineEdge {
            source: "up".into(),
            target: "down".into(),
            edge_type: EdgeType::Conditional {
                path: "verdict".into(),
                equals: serde_json::Value::String("FAIL".into()),
            },
            mode: Default::default(),
            mapping: None,
        }],
        template_id: None,
        inputs: None,
        input_schema: None,
    };
    let ready = compute_ready_set(&definition, &states);
    assert_eq!(ready, vec!["down".to_string()]);
}

#[test]
fn always_edge_can_satisfy_target_when_conditional_is_false() {
    let mut states = HashMap::new();
    states.insert("cond".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
    states.insert("always".to_string(), completed_state(r#"{"ok":true}"#));
    states.insert("down".to_string(), pending_state());
    let definition = PipelineDefinition {
        nodes: vec![
            make_thinking_node("cond"),
            make_thinking_node("always"),
            make_thinking_node("down"),
        ],
        edges: vec![
            PipelineEdge {
                source: "cond".into(),
                target: "down".into(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".into(),
                    equals: serde_json::Value::String("FAIL".into()),
                },
                mode: Default::default(),
                mapping: None,
            },
            PipelineEdge {
                source: "always".into(),
                target: "down".into(),
                edge_type: EdgeType::Always,
                mode: Default::default(),
                mapping: None,
            },
        ],
        template_id: None,
        inputs: None,
        input_schema: None,
    };

    let ready = compute_ready_set(&definition, &states);
    assert_eq!(ready, vec!["down".to_string()]);
}

#[test]
fn conditional_only_target_with_unmet_predicate_is_unreachable() {
    let mut states = HashMap::new();
    states.insert("up".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
    states.insert("down".to_string(), pending_state());
    let definition = PipelineDefinition {
        nodes: vec![make_thinking_node("up"), make_thinking_node("down")],
        edges: vec![PipelineEdge {
            source: "up".into(),
            target: "down".into(),
            edge_type: EdgeType::Conditional {
                path: "verdict".into(),
                equals: serde_json::Value::String("FAIL".into()),
            },
            mode: Default::default(),
            mapping: None,
        }],
        template_id: None,
        inputs: None,
        input_schema: None,
    };

    let unreachable = compute_unreachable_conditional_set(&definition, &states);
    assert_eq!(unreachable, vec!["down".to_string()]);
}

#[test]
fn conditional_target_with_always_fallback_is_not_unreachable() {
    let mut states = HashMap::new();
    states.insert("cond".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
    states.insert("always".to_string(), completed_state(r#"{"ok":true}"#));
    states.insert("down".to_string(), pending_state());
    let definition = PipelineDefinition {
        nodes: vec![
            make_thinking_node("cond"),
            make_thinking_node("always"),
            make_thinking_node("down"),
        ],
        edges: vec![
            PipelineEdge {
                source: "cond".into(),
                target: "down".into(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".into(),
                    equals: serde_json::Value::String("FAIL".into()),
                },
                mode: Default::default(),
                mapping: None,
            },
            PipelineEdge {
                source: "always".into(),
                target: "down".into(),
                edge_type: EdgeType::Always,
                mode: Default::default(),
                mapping: None,
            },
        ],
        template_id: None,
        inputs: None,
        input_schema: None,
    };

    let unreachable = compute_unreachable_conditional_set(&definition, &states);
    assert!(
        unreachable.is_empty(),
        "expected no unreachable nodes, got: {unreachable:?}"
    );
}
