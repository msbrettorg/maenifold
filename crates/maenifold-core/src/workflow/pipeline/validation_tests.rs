use super::*;
use crate::workflow::pipeline::{
    EdgeType, PipelineDefinition, PipelineEdge, PipelineNode, SessionRef,
};

fn make_node(id: &str) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.to_string(),
        display_label: None,
        topic: "topic".to_string(),
        total_thoughts: 1,
        analysis_type: None,
        agent: "agent".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
    }
}

fn base_definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![make_node("source"), make_node("target")],
        edges: vec![],
        inputs: None,
        template_id: None,
    }
}

fn make_prompt_node(id: &str, session_ref: Option<SessionRef>) -> PipelineNode {
    PipelineNode::Prompt {
        id: id.to_string(),
        display_label: None,
        prompt: "prompt".to_string(),
        agent: "copilot".to_string(),
        tool_mode: "default".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        session_ref,
    }
}

#[test]
fn validate_pipeline_accepts_conditional_edge_with_non_empty_path() {
    let mut definition = base_definition();
    definition.edges.push(PipelineEdge {
        source: "source".to_string(),
        target: "target".to_string(),
        edge_type: EdgeType::Conditional {
            path: "result.verdict".to_string(),
            equals: serde_json::json!("PASS"),
        },
        mapping: None,
    });

    let result = validate_pipeline(&definition);
    assert!(result.is_ok(), "expected conditional edge to validate");
}

#[test]
fn validate_pipeline_rejects_conditional_edge_with_empty_path() {
    let mut definition = base_definition();
    definition.edges.push(PipelineEdge {
        source: "source".to_string(),
        target: "target".to_string(),
        edge_type: EdgeType::Conditional {
            path: "   ".to_string(),
            equals: serde_json::json!("PASS"),
        },
        mapping: None,
    });

    let error = validate_pipeline(&definition).expect_err("expected validation error");
    assert_eq!(
        error.to_string(),
        "Conditional edge 'source -> target' field 'path' must not be empty"
    );
}

#[test]
fn validate_pipeline_rejects_conditional_edge_with_unknown_source_node() {
    let mut definition = base_definition();
    definition.edges.push(PipelineEdge {
        source: "missing".to_string(),
        target: "target".to_string(),
        edge_type: EdgeType::Conditional {
            path: "verdict".to_string(),
            equals: serde_json::json!("PASS"),
        },
        mapping: None,
    });

    let error = validate_pipeline(&definition).expect_err("expected validation error");
    assert_eq!(
        error.to_string(),
        "Conditional edge 'missing -> target' references unknown source node 'missing'"
    );
}

#[test]
fn validate_pipeline_accepts_unique_prompt_session_refs() {
    let definition = PipelineDefinition {
        nodes: vec![
            make_prompt_node(
                "node-a",
                Some(SessionRef {
                    agent: "copilot".to_string(),
                    session_id: "sess-123".to_string(),
                }),
            ),
            make_prompt_node(
                "node-b",
                Some(SessionRef {
                    agent: "copilot".to_string(),
                    session_id: "sess-456".to_string(),
                }),
            ),
        ],
        edges: vec![],
        inputs: None,
        template_id: None,
    };

    let result = validate_pipeline(&definition);
    assert!(result.is_ok(), "expected unique session refs to validate");
}

#[test]
fn validate_pipeline_rejects_duplicate_prompt_session_refs() {
    let definition = PipelineDefinition {
        nodes: vec![
            make_prompt_node(
                "node-a",
                Some(SessionRef {
                    agent: "copilot".to_string(),
                    session_id: "sess-123".to_string(),
                }),
            ),
            make_prompt_node(
                "node-b",
                Some(SessionRef {
                    agent: "copilot".to_string(),
                    session_id: "sess-123".to_string(),
                }),
            ),
        ],
        edges: vec![],
        inputs: None,
        template_id: None,
    };

    let error = validate_pipeline(&definition).expect_err("expected validation error");
    assert_eq!(
        error.to_string(),
        "Nodes 'node-a' and 'node-b' share the same sessionRef (agent: 'copilot', sessionId: 'sess-123'). Each session can only be referenced by one node."
    );
}
