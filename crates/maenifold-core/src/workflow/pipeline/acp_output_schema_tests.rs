use std::collections::HashMap;

use serde_json::{Value as JsonValue, json};

use crate::{error::Result, tools::ToolContext, util::json::OutputFormat};

use super::executor_apply::apply_node_outcome;
use super::executor_helpers::DispatchedNode;
use super::executor_nodes::{NodeExecutionRequest, build_node_execution_request};
use super::executor_orchestration::ExecutionCounts;
use super::executor_run::{
    NodeOutcome, NodeTaskResult, PromptExecutionResult, classify_node_result,
};
use super::mapping_apply::mapped_tool_args;
use super::mapping_predecessor::predecessor_context_content;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::update_status::update_pipeline_node_status;
use super::{
    EdgeDataMode, EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
};

fn output_schema() -> JsonValue {
    json!({"type":"object","required":["verdict"],"properties":{"verdict":{"enum":["PASS","FAIL"]}}})
}

fn schema_node(value: JsonValue) -> PipelineNode {
    serde_json::from_value(value).expect("ACP node with outputSchema should deserialize")
}

fn schema_prompt(id: &str) -> PipelineNode {
    schema_node(json!({
        "type":"prompt","id":id,"prompt":"return verdict","agent":"copilot",
        "model":"test-model","outputSchema":output_schema()
    }))
}

fn legacy_prompt(id: &str) -> PipelineNode {
    serde_json::from_value(json!({
        "type":"prompt","id":id,"prompt":"return prose","agent":"copilot",
        "toolMode":"read","model":"test-model","allowedTools":["memory.read"]
    }))
    .expect("legacy prompt should deserialize")
}

fn node_state(id: &str, status: NodeStatus, output: Option<&str>) -> NodeState {
    let status = match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Skipped => "skipped",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
        NodeStatus::Failed => "failed",
        NodeStatus::Retrying => "retrying",
    };
    let output = output
        .map(|value| format!("output: {value}\n"))
        .unwrap_or_default();
    serde_yml::from_str(&format!(
        "sessionId: workflow-1735689604000-{id}\nstatus: {status}\nattempts: 1\n{output}"
    ))
    .expect("node state should deserialize")
}

fn definition(nodes: Vec<PipelineNode>, edges: Vec<PipelineEdge>) -> PipelineDefinition {
    PipelineDefinition {
        nodes,
        edges,
        inputs: None,
        template_id: None,
    }
}

fn build_request_for(node: &PipelineNode) -> NodeExecutionRequest {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    build_node_execution_request(
        &context,
        &definition(vec![node.clone()], vec![]),
        &HashMap::new(),
        node,
        &DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some(format!("workflow-1735689604000-{}", node.id())),
        },
        "",
        &serde_json::Map::new(),
    )
    .expect("node execution request should build")
}

#[rustfmt::skip]
fn seed_schema_source(context: &ToolContext, session_id: &str) {
    seed_pipeline_session_with_node_sessions(context, session_id, definition(vec![schema_prompt("source")], vec![]), HashMap::from([("source".to_string(), node_state("source", NodeStatus::Active, None))]), 1);
}

#[rustfmt::skip]
fn complete_schema_source(
    context: &ToolContext,
    session_id: &str,
    conclusion: &str,
) -> Result<String> {
    update_pipeline_node_status(context, session_id, "source", "completed", Some(conclusion), Some("acp-1"), None, None, None, None, None, Some("test-model"))
}

fn node_states_from_session(context: &ToolContext, session_id: &str) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) =
        super::session_read::read_pipeline_session(context, session_id).expect("read pipeline");
    super::session_io::frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

#[test]
fn output_schema_field_round_trips_on_acp_nodes() {
    let nodes = [
        json!({"type":"prompt","id":"prompt","prompt":"return verdict","agent":"copilot","outputSchema":output_schema()}),
        json!({"type":"workflow","id":"workflow","workflowId":"test-workflow-a","acpClient":"copilot","outputSchema":output_schema()}),
        json!({"type":"sequentialThinking","id":"thinking","topic":"decide","agent":"copilot","totalThoughts":2,"outputSchema":output_schema()}),
    ];
    for value in nodes {
        let node: PipelineNode =
            serde_json::from_value(value).expect("node should accept outputSchema");
        assert_eq!(
            serde_json::to_value(node)
                .expect("node should serialize")
                .get("outputSchema"),
            Some(&output_schema())
        );
    }
}

#[test]
fn acp_node_without_output_schema_runs_as_today() {
    let NodeExecutionRequest::Prompt { session_config, .. } =
        build_request_for(&legacy_prompt("legacy"))
    else {
        panic!("legacy prompt should build a prompt execution request");
    };
    assert_eq!(
        session_config.allowed_tools.as_deref(),
        Some(&["memory.read".to_string()][..])
    );
    let outcome = classify_node_result(
        "legacy",
        NodeTaskResult::Prompt(Ok(PromptExecutionResult {
            response: "plain final answer".to_string(),
            acp_session_id: Some("acp-1".to_string()),
            input_tokens: None,
            output_tokens: None,
            model: Some("test-model".to_string()),
        })),
    );
    assert!(matches!(outcome, NodeOutcome::Completed { .. }));
}

#[test]
fn acp_node_with_output_schema_blocks_completion_until_complete_node_called() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-schema-blocks-completion";
    seed_schema_source(&context, session_id);
    let result = complete_schema_source(
        &context,
        session_id,
        "plain final answer with no complete_node call",
    );

    assert!(
        result.is_err(),
        "schema node must not complete from prose alone"
    );
    assert_eq!(
        node_states_from_session(&context, session_id)["source"].status,
        NodeStatus::Active
    );
}

#[test]
fn complete_node_with_invalid_args_returns_validation_error_to_agent() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-schema-invalid-complete";
    seed_schema_source(&context, session_id);

    let error = complete_schema_source(&context, session_id, r#"{"verdict":"MAYBE"}"#)
        .expect_err("invalid complete_node args should be returned as tool error");
    assert!(error.to_string().contains("validation"));
    let value = serde_json::to_value(&node_states_from_session(&context, session_id)["source"])
        .expect("serialize node state");
    assert!(value.get("outputArtifact").is_none());
}

#[test]
fn complete_node_with_valid_args_stores_output_artifact_and_completes_node() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-schema-valid-complete";
    seed_schema_source(&context, session_id);

    complete_schema_source(&context, session_id, r#"{"verdict":"PASS"}"#)
        .expect("valid complete_node args should complete");
    let state = &node_states_from_session(&context, session_id)["source"];
    let value = serde_json::to_value(state).expect("serialize node state");
    assert_eq!(state.status, NodeStatus::Completed);
    assert_eq!(
        value.get("outputArtifact"),
        Some(&json!({"verdict":"PASS"}))
    );
}

#[test]
fn output_artifact_is_persisted_in_node_state_markdown() {
    let state: NodeState = serde_yml::from_str(
        "sessionId: workflow-1735689604000-source\nstatus: completed\nattempts: 1\noutput: prose transcript\noutputArtifact:\n  verdict: PASS\n",
    )
    .expect("node state with outputArtifact should deserialize");

    let value = serde_json::to_value(state).expect("node state should serialize");
    assert_eq!(
        value.get("outputArtifact"),
        Some(&json!({"verdict":"PASS"}))
    );
}

#[test]
fn binding_edge_skips_verbatim_prepend_when_source_has_output_artifact() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let def = definition(
        vec![schema_prompt("source"), legacy_prompt("target")],
        vec![PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Binding,
            mapping: Some(HashMap::from([(
                "verdict".to_string(),
                "verdict".to_string(),
            )])),
        }],
    );
    let source: NodeState = serde_yml::from_str(
        "sessionId: workflow-1735689604000-source\nstatus: completed\nattempts: 1\noutput: VERBATIM SOURCE PROSE\noutputArtifact:\n  verdict: PASS\n",
    )
    .expect("node state with outputArtifact should deserialize");
    let states = HashMap::from([
        ("source".to_string(), source),
        (
            "target".to_string(),
            node_state("target", NodeStatus::Pending, None),
        ),
    ]);

    let context_text = predecessor_context_content(&context, &def, &states, "target")
        .expect("predecessor context should build");
    assert!(
        context_text
            .as_deref()
            .is_none_or(|value| !value.contains("VERBATIM SOURCE PROSE"))
    );
    let mapped = mapped_tool_args(&def, &states, "target", None, &json!({}))
        .expect("binding should map from outputArtifact, not output prose");
    assert_eq!(mapped, Some(json!({"verdict":"PASS"})));
}

#[test]
fn repair_turn_fires_once_when_no_complete_node_call_made() {
    let NodeExecutionRequest::Prompt { session_config, .. } =
        build_request_for(&schema_prompt("source"))
    else {
        panic!("schema prompt should build a prompt execution request");
    };
    let tools = session_config.allowed_tools.unwrap_or_default();
    assert_eq!(
        tools
            .iter()
            .filter(|tool| tool.as_str() == "complete_node")
            .count(),
        1
    );
    assert_eq!(
        session_config
            .config_options
            .unwrap_or_default()
            .get("outputSchemaRepairTurns"),
        Some(&json!(1)),
        "schema nodes must dispatch exactly one repair turn"
    );
}

#[test]
fn hard_fail_after_repair_turn_fails_to_validate() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-schema-hard-fail";
    seed_schema_source(&context, session_id);
    let _ = complete_schema_source(&context, session_id, "plain final answer");
    let _ = complete_schema_source(&context, session_id, r#"{"verdict":"MAYBE"}"#);

    let state = &node_states_from_session(&context, session_id)["source"];
    let value = serde_json::to_value(state).expect("serialize node state");
    assert_eq!(value.get("status"), Some(&json!("failed")));
    assert!(
        state.failure_reason.as_deref().is_some_and(
            |reason| reason.contains("ACP node did not produce valid structured output")
        )
    );
}

#[test]
fn acp_runtime_complete_node_roundtrip_smoke() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-runtime-complete-node-smoke";
    seed_schema_source(&context, session_id);

    let outcome = classify_node_result(
        "source",
        NodeTaskResult::Prompt(Ok(PromptExecutionResult {
            response: r#"{"verdict":"PASS"}"#.to_string(),
            acp_session_id: Some("acp-smoke".to_string()),
            input_tokens: Some(3),
            output_tokens: Some(5),
            model: Some("test-model".to_string()),
        })),
    );
    let mut counts = ExecutionCounts::default();
    apply_node_outcome(&context, session_id, "source", outcome, &mut counts)
        .expect("complete_node args should complete through pipeline status update");

    let state = &node_states_from_session(&context, session_id)["source"];
    let value = serde_json::to_value(state).expect("serialize node state");
    assert_eq!(state.status, NodeStatus::Completed);
    assert_eq!(
        value.get("outputArtifact"),
        Some(&json!({"verdict":"PASS"}))
    );
}

#[test]
fn acp_output_schema_complete_node_to_binding_end_to_end() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-acp-schema-complete-node-binding-e2e";
    let def = definition(
        vec![schema_prompt("source"), legacy_prompt("target")],
        vec![PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Binding,
            mapping: Some(HashMap::from([(
                "decision".to_string(),
                "verdict".to_string(),
            )])),
        }],
    );
    seed_pipeline_session_with_node_sessions(
        &context,
        session_id,
        def.clone(),
        HashMap::from([
            (
                "source".to_string(),
                node_state("source", NodeStatus::Active, None),
            ),
            (
                "target".to_string(),
                node_state("target", NodeStatus::Pending, None),
            ),
        ]),
        1,
    );

    let request = build_node_execution_request(
        &context,
        &def,
        &node_states_from_session(&context, session_id),
        &def.nodes[0],
        &DispatchedNode {
            node_id: "source".to_string(),
            session_id: Some("workflow-1735689604000-source".to_string()),
        },
        "",
        &serde_json::Map::new(),
    )
    .expect("schema source dispatch request should build");
    let NodeExecutionRequest::Prompt { session_config, .. } = request else {
        panic!("source should dispatch as an ACP prompt node");
    };
    assert_eq!(session_config.output_schema, Some(output_schema()));
    assert!(
        session_config
            .allowed_tools
            .as_deref()
            .is_some_and(|tools| tools.iter().any(|tool| tool == "complete_node")),
        "agent dispatch must advertise complete_node as an allowed completion tool"
    );

    let outcome = classify_node_result(
        "source",
        NodeTaskResult::Prompt(Ok(PromptExecutionResult {
            response: r#"{"verdict":"PASS"}"#.to_string(),
            acp_session_id: Some("acp-e2e".to_string()),
            input_tokens: Some(7),
            output_tokens: Some(11),
            model: Some("test-model".to_string()),
        })),
    );
    let mut counts = ExecutionCounts::default();
    apply_node_outcome(&context, session_id, "source", outcome, &mut counts)
        .expect("simulated complete_node args should populate artifact");

    let states = node_states_from_session(&context, session_id);
    assert_eq!(states["source"].status, NodeStatus::Completed);
    assert_eq!(
        states["source"].output_artifact,
        Some(json!({"verdict":"PASS"}))
    );
    let mapped = mapped_tool_args(&def, &states, "target", None, &json!({}))
        .expect("binding edge should consume output artifact");
    assert_eq!(mapped, Some(json!({"decision":"PASS"})));
}
