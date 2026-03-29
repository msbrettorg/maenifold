use serde_json::json;
use std::collections::HashMap;

use super::{EdgeType, PipelineEdge, PipelineNode};

#[test]
fn old_workflow_json_still_deserializes() {
    let node: PipelineNode =
        serde_json::from_value(json!({"id":"n1","workflowId":"wf","acpClient":"agent"})).unwrap();
    assert_eq!(
        node,
        PipelineNode::Workflow {
            id: "n1".into(),
            workflow_id: "wf".into(),
            acp_client: Some("agent".into()),
            prompt: None,
        
            timeout: None}
    );
}

#[test]
fn workflow_serialization_is_tagged() {
    let node = PipelineNode::Workflow {
        id: "n1".into(),
        workflow_id: "wf".into(),
        acp_client: None,
        prompt: Some("Summarize".into()),
    
        timeout: None};
    assert_eq!(
        serde_json::to_value(node).unwrap(),
        json!({"type":"workflow","id":"n1","workflowId":"wf","prompt":"Summarize"})
    );
}

#[test]
fn prompt_serialization_is_tagged() {
    let node = PipelineNode::Prompt {
        id: "n2".into(),
        prompt: "Review {{input}}".into(),
        agent: "claude".into(),
        tool_mode: "disabled".into(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    
        timeout: None};
    assert_eq!(
        serde_json::to_value(node).unwrap(),
        json!({"type":"prompt","id":"n2","prompt":"Review {{input}}","agent":"claude"})
    );
}

#[test]
fn sequential_thinking_serialization_is_tagged() {
    let node = PipelineNode::SequentialThinking {
        id: "n3".into(),
        topic: "Tradeoffs".into(),
        total_thoughts: 5,
        analysis_type: Some("architecture".into()),
        agent: "claude".into(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    
        timeout: None};
    assert_eq!(
        serde_json::to_value(node).unwrap(),
        json!({"type":"sequentialThinking","id":"n3","topic":"Tradeoffs","analysisType":"architecture","agent":"claude"})
    );
}

#[test]
fn node_without_type_and_without_workflow_id_gives_clear_error() {
    let err =
        serde_json::from_value::<PipelineNode>(json!({"id": "n99"})).expect_err("should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("n99"),
        "error should mention node id, got: {msg}"
    );
    assert!(
        msg.contains("missing 'type' field"),
        "error should mention missing type, got: {msg}"
    );
    assert!(
        msg.contains("workflow") && msg.contains("prompt") && msg.contains("tool"),
        "error should list expected types, got: {msg}"
    );
}

#[test]
fn node_with_unknown_type_gives_clear_error() {
    let err = serde_json::from_value::<PipelineNode>(json!({"id": "n42", "type": "badType"}))
        .expect_err("should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("n42"),
        "error should mention node id, got: {msg}"
    );
    assert!(
        msg.contains("unknown type") && msg.contains("badType"),
        "error should mention unknown type value, got: {msg}"
    );
}

#[test]
fn tool_node_deserializes() {
    let node: PipelineNode = serde_json::from_value(json!({
        "type": "tool",
        "id": "t1",
        "toolName": "pipeline",
        "toolArgs": { "sessionId": "pipeline-42" },
        "agent": "claude"
    }))
    .unwrap();
    assert_eq!(
        node,
        PipelineNode::Tool {
            id: "t1".into(),
            tool_name: "pipeline".into(),
            tool_args: Some(json!({"sessionId": "pipeline-42"})),
            agent: Some("claude".into()),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
        }
    );
}

#[test]
fn tool_node_deserializes_without_agent() {
    let node: PipelineNode = serde_json::from_value(json!({
        "type": "tool",
        "id": "t1",
        "toolName": "pipeline",
        "toolArgs": { "sessionId": "pipeline-42" }
    }))
    .unwrap();
    assert_eq!(
        node,
        PipelineNode::Tool {
            id: "t1".into(),
            tool_name: "pipeline".into(),
            tool_args: Some(json!({"sessionId": "pipeline-42"})),
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
        }
    );
}

#[test]
fn tool_node_serializes() {
    let node = PipelineNode::Tool {
        id: "t1".into(),
        tool_name: "pipeline".into(),
        tool_args: Some(json!({"sessionId": "pipeline-42"})),
        agent: Some("claude".into()),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    };
    assert_eq!(
        serde_json::to_value(node).unwrap(),
        json!({
            "type": "tool",
            "id": "t1",
            "toolName": "pipeline",
            "toolArgs": {"sessionId": "pipeline-42"},
            "agent": "claude"
        })
    );
}

#[test]
fn tool_node_serializes_without_agent() {
    let node = PipelineNode::Tool {
        id: "t1".into(),
        tool_name: "pipeline".into(),
        tool_args: Some(json!({"sessionId": "pipeline-42"})),
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    };
    assert_eq!(
        serde_json::to_value(node).unwrap(),
        json!({
            "type": "tool",
            "id": "t1",
            "toolName": "pipeline",
            "toolArgs": {"sessionId": "pipeline-42"}
        })
    );
}

#[test]
fn pipeline_edge_deserializes_without_mapping() {
    let edge: PipelineEdge = serde_json::from_value(json!({
        "source": "a",
        "target": "b",
        "type": "success"
    }))
    .unwrap();

    assert_eq!(
        edge,
        PipelineEdge {
            source: "a".into(),
            target: "b".into(),
            edge_type: EdgeType::Success,
            mapping: None,
        }
    );
}

#[test]
fn pipeline_edge_serializes_with_mapping() {
    let edge = PipelineEdge {
        source: "a".into(),
        target: "b".into(),
        edge_type: EdgeType::Always,
        mapping: Some(HashMap::from([
            ("title".into(), "results[0].title".into()),
            ("content".into(), "data.content".into()),
        ])),
    };

    let value = serde_json::to_value(edge).unwrap();
    assert_eq!(value["source"], "a");
    assert_eq!(value["target"], "b");
    assert_eq!(value["type"], "always");
    assert_eq!(value["mapping"]["title"], "results[0].title");
    assert_eq!(value["mapping"]["content"], "data.content");
}
