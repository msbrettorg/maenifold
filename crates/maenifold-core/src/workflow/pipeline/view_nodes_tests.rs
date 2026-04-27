use std::collections::HashMap;

use serde_json::json;
use serial_test::serial;

use crate::util::json::OutputFormat;

use super::*;
use crate::workflow::pipeline::test_support::pipeline_test_context;
use crate::workflow::pipeline::{PipelineEdge, ToolProvenance, types::EdgeType};

#[test]
#[serial]
fn pipeline_nodes_json_keeps_local_session_id_and_surfaces_provenance_links() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::SequentialThinking {
                id: "think".to_string(),
                display_label: Some("Deep Reasoning".to_string()),
                topic: "Topic".to_string(),
                total_thoughts: 3,
                analysis_type: None,
                agent: "agent-think".to_string(),
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
            },
            PipelineNode::Tool {
                id: "tool".to_string(),
                display_label: None,
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
            },
        ],
        edges: vec![PipelineEdge {
            source: "think".to_string(),
            target: "tool".to_string(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "think".to_string(),
            NodeState {
                session_id: "workflow-think-local".to_string(),
                status: NodeStatus::Pending,
                attempts: 1,
                started_at: None,
                completed_at: None,
                acp_session_id: Some("acp-think-remote".to_string()),
                thinking_session_id: Some("thinking-remote".to_string()),
                tool_provenance: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                output: None,
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        ),
        (
            "tool".to_string(),
            NodeState {
                session_id: "workflow-tool-local".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                started_at: None,
                completed_at: None,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: Some(ToolProvenance {
                    tool_name: "example.echo".to_string(),
                    resolved_args: Some(json!({ "value": 1 })),
                    response_summary: "{\"value\":1}".to_string(),
                }),
                input_tokens: None,
                output_tokens: None,
                model: None,
                output: Some("{\"value\":1}".to_string()),
                failure_reason: None,
                retry_after: None,
                retry_count: None,
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
    assert_eq!(think["displayLabel"], "Deep Reasoning");
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
    assert_eq!(think["provenance"]["detail"]["canReconnect"], true);
    assert_eq!(think["provenance"]["detail"]["adoption"]["role"], "none");
    assert_eq!(think["provenance"]["detail"]["adoption"]["color"], "none");
    assert_eq!(
        think["provenance"]["detail"]["adoption"]["perspective"],
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
    assert_eq!(tool["provenance"]["detail"]["canReconnect"], false);
    assert_eq!(tool["provenance"]["detail"]["adoption"]["role"], "none");
    assert_eq!(tool["provenance"]["detail"]["adoption"]["color"], "none");
    assert_eq!(
        tool["provenance"]["detail"]["adoption"]["perspective"],
        "none"
    );
}
