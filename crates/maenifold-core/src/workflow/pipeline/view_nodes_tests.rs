use std::collections::HashMap;

use serde_json::{Value, json};
use serial_test::serial;

use crate::util::json::OutputFormat;

use super::*;
use crate::workflow::pipeline::test_support::pipeline_test_context;
use crate::workflow::pipeline::{PipelineEdge, ToolProvenance, types::EdgeType};

fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        display_label: Some(format!("Workflow {id}")),
        workflow_id: format!("workflow-{id}"),
        acp_client: Some(format!("agent-{id}")),
        prompt: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        join_policy: None,
    }
}

fn thinking_node(id: &str) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.to_string(),
        display_label: Some(format!("Thinking {id}")),
        topic: "Topic".to_string(),
        total_thoughts: 3,
        agent: format!("agent-{id}"),
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
        join_policy: None,
    }
}

fn tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: Some(format!("Tool {id}")),
        tool_name: "example.echo".to_string(),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
        join_policy: None,
    }
}

fn node_state(session_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
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
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn sample_tool_provenance() -> ToolProvenance {
    ToolProvenance {
        tool_name: "example.echo".to_string(),
        resolved_args: Some(json!({ "value": 1 })),
        response_summary: "{\"value\":1}".to_string(),
    }
}

fn detail_object(node: &Value) -> &serde_json::Map<String, Value> {
    node["provenance"]["detail"]
        .as_object()
        .expect("detail should be an object")
}

fn single_node_json(node: PipelineNode, state: NodeState) -> Value {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![node],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = HashMap::from([(definition.nodes[0].id().to_string(), state)]);

    pipeline_nodes_json(&context, &definition, &node_states, "")
        .expect("node view should serialize")
        .into_iter()
        .next()
        .expect("single node should be returned")
}

#[test]
#[serial]
fn pipeline_nodes_json_keeps_local_session_id_and_omits_can_reconnect() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            thinking_node("think"),
            workflow_node("workflow"),
            tool_node("tool"),
        ],
        edges: vec![
            PipelineEdge {
                source: "think".to_string(),
                target: "workflow".to_string(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: None,
            },
            PipelineEdge {
                source: "workflow".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: None,
            },
        ],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "think".to_string(),
            NodeState {
                acp_session_id: Some("acp-think-remote".to_string()),
                thinking_session_id: Some("thinking-remote".to_string()),
                ..node_state("workflow-think-local", NodeStatus::Pending)
            },
        ),
        (
            "workflow".to_string(),
            NodeState {
                acp_session_id: Some("acp-workflow-remote".to_string()),
                ..node_state("workflow-local", NodeStatus::Pending)
            },
        ),
        (
            "tool".to_string(),
            NodeState {
                tool_provenance: Some(sample_tool_provenance()),
                output: Some("{\"value\":1}".to_string()),
                ..node_state("workflow-tool-local", NodeStatus::Completed)
            },
        ),
    ]);

    let nodes = pipeline_nodes_json(&context, &definition, &node_states, "")
        .expect("node view should serialize");

    let think = nodes
        .iter()
        .find(|node| node["nodeId"] == "think")
        .expect("think node");
    assert_eq!(think["sessionId"], "workflow-think-local");
    assert_eq!(think["displayLabel"], "Thinking think");
    assert_eq!(think["acpSessionId"], "acp-think-remote");
    assert_eq!(think["thinkingSessionId"], "thinking-remote");
    assert_eq!(think["provenance"]["agent"], "agent-think");
    assert_eq!(think["provenance"]["inspectable"], true);
    assert_eq!(
        think["provenance"]["detail"]["sessionId"],
        "acp-think-remote"
    );
    assert_eq!(
        think["provenance"]["detail"]["thinkingSessionId"],
        "thinking-remote"
    );
    assert!(!detail_object(think).contains_key("canReconnect"));
    assert_eq!(think["provenance"]["detail"]["adoption"]["role"], "none");
    assert_eq!(think["provenance"]["detail"]["adoption"]["color"], "none");
    assert_eq!(
        think["provenance"]["detail"]["adoption"]["perspective"],
        "none"
    );

    let workflow = nodes
        .iter()
        .find(|node| node["nodeId"] == "workflow")
        .expect("workflow node");
    assert_eq!(workflow["sessionId"], "workflow-local");
    assert_eq!(workflow["acpSessionId"], "acp-workflow-remote");
    assert_eq!(workflow["provenance"]["agent"], "agent-workflow");
    assert_eq!(workflow["provenance"]["inspectable"], true);
    assert_eq!(
        workflow["provenance"]["detail"]["sessionId"],
        "acp-workflow-remote"
    );
    assert_eq!(
        workflow["provenance"]["detail"]["thinkingSessionId"],
        Value::Null
    );
    assert!(!detail_object(workflow).contains_key("canReconnect"));
    assert_eq!(workflow["provenance"]["detail"]["adoption"]["role"], "none");
    assert_eq!(
        workflow["provenance"]["detail"]["adoption"]["color"],
        "none"
    );
    assert_eq!(
        workflow["provenance"]["detail"]["adoption"]["perspective"],
        "none"
    );

    let tool = nodes
        .iter()
        .find(|node| node["nodeId"] == "tool")
        .expect("tool node");
    assert_eq!(tool["sessionId"], "workflow-tool-local");
    assert_eq!(tool["toolProvenance"]["toolName"], "example.echo");
    assert_eq!(tool["toolProvenance"]["resolvedArgs"]["value"], 1);
    assert_eq!(tool["toolProvenance"]["responseSummary"], "{\"value\":1}");
    assert_eq!(tool["provenance"]["agent"], Value::Null);
    assert_eq!(tool["provenance"]["inspectable"], true);
    assert_eq!(tool["provenance"]["detail"]["sessionId"], Value::Null);
    assert_eq!(
        tool["provenance"]["detail"]["thinkingSessionId"],
        Value::Null
    );
    assert!(!detail_object(tool).contains_key("canReconnect"));
    assert_eq!(tool["provenance"]["detail"]["adoption"]["role"], "none");
    assert_eq!(tool["provenance"]["detail"]["adoption"]["color"], "none");
    assert_eq!(
        tool["provenance"]["detail"]["adoption"]["perspective"],
        "none"
    );
}

#[test]
#[serial]
fn pipeline_nodes_json_inspectable_uses_acp_thinking_and_tool_union() {
    let cases = [
        ("none", false, false, false, false),
        ("acp-only", true, false, false, true),
        ("thinking-only", false, true, false, true),
        ("tool-only", false, false, true, true),
        ("acp-and-thinking", true, true, false, true),
        ("acp-and-tool", true, false, true, true),
        ("thinking-and-tool", false, true, true, true),
        ("all-three", true, true, true, true),
    ];

    for (label, has_acp, has_thinking, has_tool_provenance, expected_inspectable) in cases {
        let mut state = node_state(&format!("workflow-{label}"), NodeStatus::Pending);
        if has_acp {
            state.acp_session_id = Some(format!("00000000-0000-0000-0000-{label:0>12}"));
        }
        if has_thinking {
            state.thinking_session_id = Some(format!("thinking-{label}"));
        }
        if has_tool_provenance {
            state.tool_provenance = Some(sample_tool_provenance());
        }

        let node_json = single_node_json(workflow_node(label), state);
        assert_eq!(
            node_json["provenance"]["inspectable"],
            json!(expected_inspectable),
            "inspectable mismatch for case {label}"
        );
    }
}
