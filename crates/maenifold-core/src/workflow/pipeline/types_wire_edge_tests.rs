use serde_json::json;

use super::super::types::{PipelineDefinition, PipelineNode, RetryPolicy, SessionRef};

#[test]
fn missing_node_type_returns_error() {
    let error = serde_json::from_value::<PipelineNode>(json!({
        "id": "prompt",
        "prompt": "Say hi",
        "agent": "agent"
    }))
    .expect_err("missing type should fail");

    let message = error.to_string();
    assert!(message.contains("missing 'type' field"));
    assert!(message.contains("workflow, prompt, sequentialThinking, tool"));
}

#[test]
fn unknown_node_type_lists_known_types() {
    let error = serde_json::from_value::<PipelineNode>(json!({
        "id": "node",
        "type": "quantum",
        "prompt": "Say hi",
        "agent": "agent"
    }))
    .expect_err("unknown type should fail");

    assert_eq!(
        error.to_string(),
        "Node 'node': unknown type 'quantum'. Expected one of: workflow, prompt, sequentialThinking, tool"
    );
}

#[test]
fn retry_policy_zero_max_attempts_is_accepted() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "tool",
        "type": "tool",
        "toolName": "GetConfig",
        "retryMaxAttempts": 0,
        "retryBackoffMinMs": 1,
        "retryBackoffMaxMs": 5,
        "retryJitter": false
    }))
    .expect("deserialize tool node with zero retry attempts");

    assert_eq!(
        node.retry_policy(),
        Some(&RetryPolicy {
            max_attempts: 0,
            backoff_min_ms: 1,
            backoff_max_ms: 5,
            jitter: false,
        })
    );
}

#[test]
fn session_ref_is_ignored_for_workflow_and_tool_serialization() {
    let workflow: PipelineNode = serde_json::from_value(json!({
        "id": "workflow-node",
        "type": "workflow",
        "workflowId": "wf-123",
        "sessionRef": {
            "agent": "agent",
            "sessionId": "session-123"
        }
    }))
    .expect("deserialize workflow node");
    let tool: PipelineNode = serde_json::from_value(json!({
        "id": "tool-node",
        "type": "tool",
        "toolName": "GetConfig",
        "sessionRef": {
            "agent": "agent",
            "sessionId": "session-123"
        }
    }))
    .expect("deserialize tool node");

    let workflow_value = serde_json::to_value(workflow).expect("serialize workflow node");
    let tool_value = serde_json::to_value(tool).expect("serialize tool node");

    assert!(workflow_value.get("sessionRef").is_none());
    assert!(tool_value.get("sessionRef").is_none());
}

#[test]
fn prompt_node_round_trips_session_ref() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "prompt",
        "type": "prompt",
        "prompt": "Say hi",
        "agent": "agent",
        "sessionRef": {
            "agent": "agent",
            "sessionId": "session-123"
        }
    }))
    .expect("deserialize prompt node");

    match &node {
        PipelineNode::Prompt { session_ref, .. } => assert_eq!(
            session_ref.as_ref(),
            Some(&SessionRef {
                agent: "agent".to_string(),
                session_id: "session-123".to_string(),
            })
        ),
        other => panic!("expected prompt node, got {other:?}"),
    }

    let value = serde_json::to_value(node).expect("serialize prompt node");
    assert_eq!(value["sessionRef"]["agent"], json!("agent"));
    assert_eq!(value["sessionRef"]["sessionId"], json!("session-123"));
}

#[test]
fn prompt_node_rejects_session_ref_agent_mismatch() {
    let error = serde_json::from_value::<PipelineNode>(json!({
        "id": "prompt",
        "type": "prompt",
        "prompt": "Say hi",
        "agent": "agent-a",
        "sessionRef": {
            "agent": "agent-b",
            "sessionId": "session-123"
        }
    }))
    .expect_err("sessionRef.agent mismatch should fail");

    assert!(
        error
            .to_string()
            .contains("sessionRef.agent ('agent-b') must match node agent ('agent-a')")
    );
}

#[test]
fn pipeline_definition_round_trips_prompt_session_ref() {
    let json = r#"{
        "nodes": [
            {
                "id": "prompt-node",
                "type": "prompt",
                "prompt": "Say hi",
                "agent": "test-agent",
                "sessionRef": {
                    "agent": "test-agent",
                    "sessionId": "sess-123"
                }
            },
            {
                "id": "tool-node",
                "type": "tool",
                "toolName": "GetConfig"
            }
        ],
        "edges": [
            {
                "source": "prompt-node",
                "target": "tool-node",
                "type": "success"
            }
        ]
    }"#;

    let pipeline: PipelineDefinition =
        serde_json::from_str(json).expect("deserialize pipeline definition");

    match &pipeline.nodes[0] {
        PipelineNode::Prompt { session_ref, .. } => assert_eq!(
            session_ref.as_ref(),
            Some(&SessionRef {
                agent: "test-agent".to_string(),
                session_id: "sess-123".to_string(),
            })
        ),
        other => panic!("expected prompt node, got {other:?}"),
    }

    let serialized = serde_json::to_string(&pipeline).expect("serialize pipeline definition");
    assert!(serialized.contains("\"sessionRef\""));
}

#[test]
fn pipeline_definition_without_session_ref_deserializes_cleanly() {
    let json = r#"{
        "nodes": [
            {
                "id": "prompt-node",
                "type": "prompt",
                "prompt": "Say hi",
                "agent": "agent"
            },
            {
                "id": "tool-node",
                "type": "tool",
                "toolName": "GetConfig"
            }
        ],
        "edges": [
            {
                "source": "prompt-node",
                "target": "tool-node",
                "type": "success"
            }
        ]
    }"#;

    let pipeline: PipelineDefinition =
        serde_json::from_str(json).expect("deserialize pipeline definition without sessionRef");

    match &pipeline.nodes[0] {
        PipelineNode::Prompt { session_ref, .. } => assert!(session_ref.is_none()),
        other => panic!("expected prompt node, got {other:?}"),
    }
}

#[test]
fn pipeline_definition_rejects_session_ref_agent_mismatch() {
    let json = r#"{
        "nodes": [
            {
                "id": "prompt-node",
                "type": "prompt",
                "prompt": "Say hi",
                "agent": "agent-a",
                "sessionRef": {
                    "agent": "agent-b",
                    "sessionId": "session-123"
                }
            }
        ],
        "edges": []
    }"#;

    let error = serde_json::from_str::<PipelineDefinition>(json)
        .expect_err("sessionRef.agent mismatch should fail");

    assert!(error.to_string().contains("must match"));
}
