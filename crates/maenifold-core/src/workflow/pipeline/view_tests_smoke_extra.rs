use crate::mcp::schema::mcp_tool_from_registry_name;
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

use super::handler::pipeline;
use super::test_support::{
    pipeline_test_context, seed_pipeline_session_with_workflows, seed_pipeline_view_session,
};
use super::view::build_pipeline_view_result;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};

#[test]
#[serial(config)]
fn pipeline_view_smoke_top_level_fields_ready_sets_and_missing_session() {
    // Covers V-01, V-09, V-10, and V-11.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("root", "test-workflow-a"),
            workflow_node("cleanup", "test-workflow-b"),
            workflow_node("child", "test-workflow-b"),
        ],
        edges: vec![
            edge("root", "child", EdgeType::Success),
            edge("root", "cleanup", EdgeType::Always),
        ],
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            node_state("workflow-root", NodeStatus::Completed, 1, Some("done")),
        ),
        (
            "cleanup".to_string(),
            node_state("workflow-cleanup", NodeStatus::Pending, 0, None),
        ),
        (
            "child".to_string(),
            node_state("workflow-child", NodeStatus::Pending, 0, None),
        ),
    ]);
    let result = build_pipeline_view_result(
        &context,
        "pipeline-smoke-top-level",
        &definition,
        &node_states,
        Some("ambient"),
    )
    .expect("build pipeline view");

    for key in [
        "pipelineId",
        "status",
        "nodes",
        "edges",
        "ready",
        "active",
        "completed",
        "mermaid",
        "running",
    ] {
        assert!(result.get(key).is_some(), "missing top-level field {key}");
    }

    assert_eq!(result["pipelineId"], "pipeline-smoke-top-level");
    assert_eq!(result["ready"], json!(["cleanup", "child"]));
    assert_eq!(result["active"], json!([]));
    assert_eq!(result["completed"], json!(["root"]));
    assert_eq!(result["running"], Value::Bool(false));

    let missing = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-missing-view", "view": true }),
    )
    .expect_err("missing pipeline should fail");
    assert!(missing.to_string().contains("pipeline-missing-view"));
}

#[test]
#[serial(config)]
fn pipeline_view_smoke_variant_specific_fields_and_mermaid_rendering() {
    // Covers V-02, V-03, V-04, V-05, V-06, V-07, and V-08.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "workflow".into(),
                workflow_id: "test-workflow-a".into(),
                acp_client: Some("workflow-agent".into()),
                prompt: Some("Inspect upstream output".into()),
                timeout: Some(120),
            },
            PipelineNode::Prompt {
                id: "prompt".into(),
                prompt: "Write operator summary".into(),
                agent: "prompt-agent".into(),
                tool_mode: "required".into(),
                model: Some("gpt-5.4-mini".into()),
                thought_level: Some("high".into()),
                allowed_tools: Some(vec!["ReadMemory".into()]),
                max_turns: Some(4),
                timeout: Some(30),
                agent_config: None,
            },
            PipelineNode::SequentialThinking {
                id: "think".into(),
                topic: "diagnose pipeline".into(),
                total_thoughts: 2,
                analysis_type: Some("diagnostic".into()),
                agent: "thinking-agent".into(),
                model: Some("claude-sonnet-4.6".into()),
                thought_level: Some("deep".into()),
                allowed_tools: None,
                max_turns: Some(5),
                timeout: Some(45),
                agent_config: None,
            },
            PipelineNode::Tool {
                id: "tool".into(),
                tool_name: "ReadMemory".into(),
                tool_args: Some(json!({ "identifier": "memory://build/test" })),
                agent: Some("tool-agent".into()),
                model: Some("gpt-5.2".into()),
                thought_level: Some("low".into()),
                allowed_tools: Some(vec!["ReadMemory".into(), "ListMemories".into()]),
                max_turns: Some(3),
                agent_config: None,
            },
        ],
        edges: vec![
            edge("workflow", "prompt", EdgeType::Success),
            edge("prompt", "think", EdgeType::Always),
            edge("think", "tool", EdgeType::Success),
        ],
    };
    let node_states = HashMap::from([
        (
            "workflow".to_string(),
            node_state("workflow-session", NodeStatus::Completed, 1, Some("done")),
        ),
        (
            "prompt".to_string(),
            node_state("prompt-session", NodeStatus::Abandoned, 2, None),
        ),
        (
            "think".to_string(),
            node_state("thinking-session", NodeStatus::Ready, 0, None),
        ),
        (
            "tool".to_string(),
            node_state("tool-session", NodeStatus::Stuck, 1, None),
        ),
    ]);
    let result = build_pipeline_view_result(
        &context,
        "pipeline-smoke-variants",
        &definition,
        &node_states,
        None,
    )
    .expect("build pipeline view");
    let nodes = result["nodes"].as_array().expect("nodes array");

    assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
    assert_eq!(nodes[0]["timeout"], 120);
    assert_eq!(nodes[1]["toolMode"], "required");
    assert_eq!(nodes[1]["maxTurns"], 4);
    assert_eq!(nodes[2]["topic"], "diagnose pipeline");
    assert_eq!(nodes[2]["totalThoughts"], 2);
    assert_eq!(nodes[3]["toolName"], "ReadMemory");
    assert_eq!(
        nodes[3]["toolArgs"],
        json!({ "identifier": "memory://build/test" })
    );

    let mermaid = result["mermaid"].as_str().expect("mermaid");
    assert!(mermaid.contains("workflow[\"✅ wf: test-workflow-a\"]"));
    assert!(mermaid.contains("prompt(\"❌ prompt: Write operator summary...\")"));
    assert!(mermaid.contains("think{{\"🟡 think: diagnose pipeline\"}}"));
    assert!(mermaid.contains("tool>🚫 tool: ReadMemory]"));
    assert!(mermaid.contains("workflow --> prompt"));
    assert!(mermaid.contains("prompt -.-> think"));
}

#[test]
#[serial(config)]
fn pipeline_list_and_learn_smoke_empty_then_seeded_sessions() {
    // Covers LS-01, LS-02, LS-03, and H-01.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let empty = pipeline(&context, &json!({ "list": true })).expect("empty list");
    let empty_response: Value = serde_json::from_str(&empty).expect("parse empty list");
    assert!(
        empty_response["data"]["pipelines"]
            .as_array()
            .expect("pipelines")
            .is_empty()
    );

    let completed_states = HashMap::from([(
        "root".to_string(),
        node_state("workflow-a-root", NodeStatus::Completed, 1, None),
    )]);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-1735689600701",
        PipelineDefinition {
            nodes: vec![workflow_node("root", "test-workflow-a")],
            edges: vec![],
        },
        completed_states,
        2,
    );
    seed_pipeline_view_session(&context, "pipeline-1735689600700", "pipeline");

    let listed = pipeline(&context, &json!({ "list": true })).expect("seeded list");
    let listed_response: Value = serde_json::from_str(&listed).expect("parse seeded list");
    let pipelines = listed_response["data"]["pipelines"]
        .as_array()
        .expect("pipelines");
    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0]["pipelineId"], "pipeline-1735689600700");
    assert_eq!(pipelines[1]["pipelineId"], "pipeline-1735689600701");
    assert_eq!(pipelines[0]["completedCount"], 1);
    assert_eq!(pipelines[1]["completedCount"], 1);

    let learn = pipeline(&context, &json!({ "learn": true })).expect("learn output");
    let learn_response: Value = serde_json::from_str(&learn).expect("parse learn response");
    assert!(
        learn_response["data"]
            .as_str()
            .expect("help")
            .contains("claim ready nodes for execution")
    );
}

#[test]
#[serial(config)]
fn pipeline_alias_registration_smoke() {
    // Covers H-02.
    let registry = ToolRegistry::new();
    assert!(registry.contains("Pipeline"));
    assert!(registry.contains("pipeline"));
    assert!(registry.canonical_names().contains(&"Pipeline"));
}

#[test]
#[serial(config)]
fn pipeline_schema_smoke_variants_required_fields_and_edge_enums() {
    // Covers SCH-01, SCH-02, and SCH-03.
    let schema = mcp_tool_from_registry_name("Pipeline")
        .expect("pipeline schema")
        .input_schema;
    let definition = schema["properties"]["definition"].clone();
    let variants = definition["properties"]["nodes"]["items"]["oneOf"]
        .as_array()
        .expect("node variants");

    assert_eq!(variants.len(), 4);
    assert_eq!(
        variants
            .iter()
            .map(|variant| {
                variant["properties"]["type"]["enum"][0]
                    .as_str()
                    .expect("type")
            })
            .collect::<Vec<_>>(),
        vec!["workflow", "prompt", "sequentialThinking", "tool"]
    );

    for (node_type, required_field) in [
        ("workflow", "workflowId"),
        ("prompt", "prompt"),
        ("sequentialThinking", "totalThoughts"),
        ("tool", "toolName"),
    ] {
        let required = variants
            .iter()
            .find(|variant| variant["properties"]["type"]["enum"][0] == node_type)
            .and_then(|variant| variant.get("required"))
            .and_then(Value::as_array)
            .expect("required array");
        assert!(required.contains(&Value::String(required_field.to_string())));
    }

    for container in ["definition", "graft", "join"] {
        let edge_enum = schema["properties"][container]["properties"]["edges"]["items"]
            ["properties"]["type"]["enum"]
            .as_array()
            .expect("edge enum");
        assert_eq!(edge_enum, &vec![json!("success"), json!("always")]);
    }
}

fn workflow_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: workflow_id.to_string(),
        acp_client: Some("workflow-agent".to_string()),
        prompt: None,
        timeout: None,
    }
}

fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.to_string(),
        target: target.to_string(),
        edge_type,
        mapping: None,
    }
}

fn node_state(
    session_id: &str,
    status: NodeStatus,
    attempts: u32,
    output: Option<&str>,
) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status,
        attempts,
        thinking_session_id: None,
        output: output.map(str::to_string),
    }
}
