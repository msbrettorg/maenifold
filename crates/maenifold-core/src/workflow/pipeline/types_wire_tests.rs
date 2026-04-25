use serde_json::json;

use super::{
    super::types::PipelineNode,
    PipelineNodeWire,
};

#[test]
fn tool_node_deserialization_preserves_timeout() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "tool",
        "type": "tool",
        "toolName": "GetConfig",
        "timeout": 21
    }))
    .expect("deserialize tool node");

    match node {
        PipelineNode::Tool { timeout, .. } => assert_eq!(timeout, Some(21)),
        other => panic!("expected tool node, got {other:?}"),
    }
}

#[test]
fn tool_node_serialization_emits_timeout() {
    let node = PipelineNode::Tool {
        id: "tool".to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: Some(21),
        agent_config: None,
        retry_policy: None,
    };

    let value = serde_json::to_value(node).expect("serialize tool node");

    assert_eq!(value["timeout"], json!(21));
}

#[test]
fn prompt_node_round_trips_display_label() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "prompt",
        "type": "prompt",
        "displayLabel": "Prompt label",
        "prompt": "Say hi",
        "agent": "agent"
    }))
    .expect("deserialize prompt node");

    match &node {
        PipelineNode::Prompt { display_label, .. } => {
            assert_eq!(display_label.as_deref(), Some("Prompt label"));
        }
        other => panic!("expected prompt node, got {other:?}"),
    }

    let value = serde_json::to_value(node).expect("serialize prompt node");
    assert_eq!(value["displayLabel"], json!("Prompt label"));
}

#[test]
fn prompt_node_round_trips_cwd() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "prompt",
        "type": "prompt",
        "prompt": "Say hi",
        "agent": "agent",
        "cwd": "/tmp/workspace"
    }))
    .expect("deserialize prompt node");

    match &node {
        PipelineNode::Prompt { cwd, .. } => {
            assert_eq!(
                cwd.as_deref().and_then(|path| path.to_str()),
                Some("/tmp/workspace")
            );
        }
        other => panic!("expected prompt node, got {other:?}"),
    }

    let value = serde_json::to_value(node).expect("serialize prompt node");
    assert_eq!(value["cwd"], json!("/tmp/workspace"));
}

#[test]
fn test_workflow_wire_round_trip_with_agent_config() {
    let json = r#"{
        "id": "workflow-node",
        "type": "workflow",
        "workflowId": "wf-123",
        "acpClient": "acp-client",
        "prompt": "Run workflow",
        "timeout": 30,
        "agentConfig": {
            "model": "gpt-5",
            "mode": "chat",
            "thoughtLevel": "high",
            "configOptions": {
                "temperature": 0.2,
                "stream": true
            }
        }
    }"#;

    let wire: PipelineNodeWire = serde_json::from_str(json).expect("deserialize workflow wire");
    let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");
    let wire = PipelineNodeWire::from(node);
    let value = serde_json::to_value(&wire).expect("serialize workflow wire");

    assert_eq!(value["workflowId"], json!("wf-123"));
    assert_eq!(value["acpClient"], json!("acp-client"));
    assert_eq!(value["agentConfig"]["model"], json!("gpt-5"));
    assert_eq!(value["agentConfig"]["thoughtLevel"], json!("high"));
}

#[test]
fn test_workflow_wire_without_agent_config() {
    let json = r#"{
        "id": "workflow-node",
        "type": "workflow",
        "workflowId": "wf-123",
        "acpClient": "acp-client"
    }"#;

    let wire: PipelineNodeWire =
        serde_json::from_str(json).expect("deserialize workflow wire without agent config");
    let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");

    match node {
        PipelineNode::Workflow { agent_config, .. } => assert!(agent_config.is_none()),
        other => panic!("expected workflow node, got {other:?}"),
    }
}

#[test]
fn test_workflow_wire_without_acp_client() {
    let json = r#"{
        "id": "workflow-node",
        "type": "workflow",
        "workflowId": "wf-123"
    }"#;

    let wire: PipelineNodeWire =
        serde_json::from_str(json).expect("deserialize workflow wire without acp client");
    let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");

    match node {
        PipelineNode::Workflow { acp_client, .. } => assert!(acp_client.is_none()),
        other => panic!("expected workflow node, got {other:?}"),
    }
}

#[test]
fn prompt_node_round_trips_adopt_and_requires_role() {
    let node: PipelineNode = serde_json::from_value(json!({
        "id": "prompt",
        "type": "prompt",
        "prompt": "Say hi",
        "agent": "agent",
        "adopt": {
            "role": "architect",
            "color": "blue",
            "perspective": "systems"
        },
        "requiresRole": "architect"
    }))
    .expect("deserialize prompt node");

    match &node {
        PipelineNode::Prompt {
            adopt,
            requires_role,
            ..
        } => {
            let adopt = adopt.as_ref().expect("adopt");
            assert_eq!(adopt.role.as_deref(), Some("architect"));
            assert_eq!(adopt.color.as_deref(), Some("blue"));
            assert_eq!(adopt.perspective.as_deref(), Some("systems"));
            assert_eq!(requires_role.as_deref(), Some("architect"));
        }
        other => panic!("expected prompt node, got {other:?}"),
    }

    let value = serde_json::to_value(node).expect("serialize prompt node");
    assert_eq!(value["adopt"]["role"], json!("architect"));
    assert_eq!(value["adopt"]["color"], json!("blue"));
    assert_eq!(value["adopt"]["perspective"], json!("systems"));
    assert_eq!(value["requiresRole"], json!("architect"));
}
