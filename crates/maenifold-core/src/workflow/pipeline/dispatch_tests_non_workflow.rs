use serde_json::json;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use super::support::{
    dispatch_data, pipeline_context, read_pipeline_states, read_session,
    seed_pipeline_session_with_node_sessions,
};

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_prompt_node_metadata_without_workflow_activation() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600480";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Prompt {
            id: "prompt-node".into(),
            prompt: "Summarize the dispatch result".into(),
            agent: "prompt-agent".into(),
            tool_mode: "default".into(),
            agent_config: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,

            timeout: None,
        }],
        edges: vec![],
    };
    let node_states = HashMap::from([node_state(
        "prompt-node",
        "workflow-1735689600480-prompt-node",
    )]);
    seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);
    let data = dispatch_data(&context, pipeline_id);
    assert_eq!(
        data["dispatched"],
        json!([{
            "nodeId": "prompt-node",
            "sessionId": "workflow-1735689600480-prompt-node",
            "type": "prompt",
            "agent": "prompt-agent",
        }])
    );
    assert!(data["dispatched"][0].get("workflowId").is_none());
    assert_eq!(
        read_pipeline_states(&context, pipeline_id)["prompt-node"].status,
        NodeStatus::Active
    );
    let (frontmatter, content) = read_session(&context, "workflow-1735689600480-prompt-node");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert!(!content.contains("## Step"));
}

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_sequential_thinking_metadata_without_workflow_activation() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600490";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::SequentialThinking {
            id: "thought-node".into(),
            topic: "Dispatch branching".into(),
            total_thoughts: 3,
            analysis_type: Some("architecture".into()),
            agent: "thinking-agent".into(),
            agent_config: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,

            timeout: None,
        }],
        edges: vec![],
    };
    let node_states = HashMap::from([node_state(
        "thought-node",
        "workflow-1735689600490-thought-node",
    )]);
    seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);
    let data = dispatch_data(&context, pipeline_id);
    assert_eq!(
        data["dispatched"],
        json!([{
            "nodeId": "thought-node",
            "sessionId": "workflow-1735689600490-thought-node",
            "type": "sequentialThinking",
            "agent": "thinking-agent",
        }])
    );
    assert!(data["dispatched"][0].get("workflowId").is_none());
    let (frontmatter, content) = read_session(&context, "workflow-1735689600490-thought-node");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert!(!content.contains("## Step"));
}

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_tool_metadata_without_workflow_activation() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600495";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Tool {
            id: "tool-node".into(),
            tool_name: "ReadMemory".into(),
            tool_args: Some(json!({"identifier": "memory://build/test"})),
            agent: None,
            agent_config: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
        }],
        edges: vec![],
    };
    let node_states = HashMap::from([node_state("tool-node", "workflow-1735689600495-tool-node")]);
    seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);
    let data = dispatch_data(&context, pipeline_id);
    assert_eq!(
        data["dispatched"],
        json!([{
            "nodeId": "tool-node",
            "sessionId": "workflow-1735689600495-tool-node",
            "type": "tool",
            "toolName": "ReadMemory",
            "toolArgs": {"identifier": "memory://build/test"},
        }])
    );
    assert!(data["dispatched"][0].get("workflowId").is_none());
}

fn node_state(id: &str, session_id: &str) -> (String, NodeState) {
    (
        id.into(),
        NodeState {
            session_id: session_id.into(),
            status: NodeStatus::Pending,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}
