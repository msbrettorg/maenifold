use std::collections::HashMap;

use serde_json::{Value as JsonValue, json};

use crate::util::json::OutputFormat;

use super::executor_helpers::DispatchedNode;
use super::executor_nodes::{NodeExecutionRequest, build_node_execution_request};
use super::test_support::pipeline_test_context;
use super::{
    EdgeDataMode, EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    validate_pipeline,
};

fn input_schema() -> JsonValue {
    json!({
        "type": "object",
        "required": ["bar"],
        "additionalProperties": false,
        "properties": {
            "bar": { "type": "string" }
        }
    })
}

fn output_schema() -> JsonValue {
    json!({
        "type": "object",
        "required": ["verdict"],
        "properties": {
            "verdict": { "type": "string" }
        }
    })
}

fn node_from(value: JsonValue) -> PipelineNode {
    serde_json::from_value(value).expect("pipeline node should deserialize")
}

fn source_node() -> PipelineNode {
    node_from(json!({
        "type": "prompt",
        "id": "source",
        "prompt": "produce structured output",
        "agent": "copilot",
        "model": "test-model",
        "outputSchema": output_schema()
    }))
}

fn target_node_with_input_schema() -> PipelineNode {
    node_from(json!({
        "type": "prompt",
        "id": "target",
        "prompt": "consume structured input",
        "agent": "copilot",
        "model": "test-model",
        "inputSchema": input_schema()
    }))
}

fn target_node_without_input_schema() -> PipelineNode {
    node_from(json!({
        "type": "prompt",
        "id": "target",
        "prompt": "consume legacy context",
        "agent": "copilot",
        "model": "test-model"
    }))
}

fn definition(nodes: Vec<PipelineNode>, edges: Vec<PipelineEdge>) -> PipelineDefinition {
    PipelineDefinition {
        nodes,
        edges,
        inputs: None,
        input_schema: None,
        template_id: None,
    }
}

fn binding_edge(target_path: &str, source_path: &str) -> PipelineEdge {
    PipelineEdge {
        source: "source".to_string(),
        target: "target".to_string(),
        edge_type: EdgeType::Success,
        mode: EdgeDataMode::Binding,
        mapping: Some(HashMap::from([(
            target_path.to_string(),
            source_path.to_string(),
        )])),
    }
}

fn node_state(id: &str, status: NodeStatus, extra_yaml: &str) -> NodeState {
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
    serde_yml::from_str(&format!(
        "sessionId: workflow-1735689605000-{id}\nstatus: {status}\nattempts: 1\n{extra_yaml}"
    ))
    .expect("node state should deserialize")
}

fn build_target_request(
    definition: &PipelineDefinition,
) -> crate::error::Result<NodeExecutionRequest> {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let states = HashMap::from([
        (
            "source".to_string(),
            node_state(
                "source",
                NodeStatus::Completed,
                "output: source prose must not be interpolated\noutputArtifact:\n  verdict: PASS\n",
            ),
        ),
        (
            "target".to_string(),
            node_state("target", NodeStatus::Pending, ""),
        ),
    ]);
    build_node_execution_request(
        &context,
        definition,
        &states,
        &definition.nodes[1],
        &DispatchedNode {
            node_id: "target".to_string(),
            session_id: Some("workflow-1735689605000-target".to_string()),
        },
        "ambient instructions",
        &serde_json::Map::new(),
    )
}

#[test]
fn input_schema_field_round_trips_on_acp_nodes() {
    let nodes = [
        json!({"type":"prompt","id":"prompt","prompt":"use input","agent":"copilot","inputSchema":input_schema()}),
        json!({"type":"workflow","id":"workflow","workflowId":"test-workflow-a","acpClient":"copilot","inputSchema":input_schema()}),
        json!({"type":"sequentialThinking","id":"thinking","topic":"decide","agent":"copilot","totalThoughts":2,"inputSchema":input_schema()}),
    ];
    for value in nodes {
        let node: PipelineNode =
            serde_json::from_value(value).expect("ACP node should accept inputSchema");
        assert_eq!(
            serde_json::to_value(node)
                .expect("node should serialize")
                .get("inputSchema"),
            Some(&input_schema())
        );
    }
}

#[test]
fn input_artifact_populated_when_input_schema_declared() {
    let _node = target_node_with_input_schema();
    let state = node_state(
        "target",
        NodeStatus::Active,
        "inputArtifact:\n  bar: PASS\n",
    );

    let value = serde_json::to_value(state).expect("node state should serialize");
    assert_eq!(value.get("inputArtifact"), Some(&json!({"bar":"PASS"})));
}

#[test]
fn runtime_inputs_envelope_appended_to_prompt_under_known_heading() {
    let def = definition(
        vec![source_node(), target_node_with_input_schema()],
        vec![binding_edge("inputs.bar", "verdict")],
    );

    let NodeExecutionRequest::Prompt { prompt, .. } =
        build_target_request(&def).expect("target request should build")
    else {
        panic!("target should dispatch as prompt");
    };

    assert_eq!(prompt.matches("## Runtime inputs").count(), 1);
    assert!(prompt.contains("```json"));
    assert!(prompt.contains("\"bar\""));
    assert!(prompt.contains("\"PASS\""));
    assert!(
        prompt.trim_end().ends_with("```"),
        "runtime inputs envelope must be appended last"
    );
}

#[test]
fn binding_edge_target_input_path_validated_against_target_input_schema() {
    let def = definition(
        vec![source_node(), target_node_with_input_schema()],
        vec![binding_edge("inputs.foo", "verdict")],
    );

    let error =
        validate_pipeline(&def).expect_err("unknown target input path should reject definition");
    let message = error.to_string();
    assert!(
        message.contains("inputs.foo"),
        "error should mention invalid target path, got {message}"
    );
}

#[test]
fn missing_required_input_after_bindings_fails_node_dispatch_with_clear_error() {
    let def = definition(
        vec![source_node(), target_node_with_input_schema()],
        vec![binding_edge("inputs.foo", "verdict")],
    );

    let error = build_target_request(&def)
        .expect_err("dispatch must fail before ACP when required input is missing");
    let message = error.to_string();
    assert!(
        message.contains("bar"),
        "error should mention bar, got {message}"
    );
}

#[test]
fn acp_node_without_input_schema_runs_as_today_no_envelope() {
    let def = definition(
        vec![source_node(), target_node_without_input_schema()],
        vec![],
    );

    let NodeExecutionRequest::Prompt { prompt, .. } =
        build_target_request(&def).expect("legacy target request should build")
    else {
        panic!("legacy target should dispatch as prompt");
    };

    assert!(!prompt.contains("Runtime inputs"));
}

#[test]
fn binding_edge_into_acp_target_without_input_schema_rejected_at_validation() {
    let def = definition(
        vec![source_node(), target_node_without_input_schema()],
        vec![binding_edge("inputs.bar", "verdict")],
    );

    let error = validate_pipeline(&def)
        .expect_err("binding into ACP target without inputSchema must reject definition");
    let message = error.to_string();
    assert!(
        message.contains("inputSchema") && message.contains("target"),
        "error should clearly identify missing target inputSchema, got {message}"
    );
}

#[test]
fn input_schema_with_no_bindings_validates_empty_and_fails_when_required_fields_missing() {
    let def = definition(
        vec![source_node(), target_node_with_input_schema()],
        vec![],
    );

    let error = build_target_request(&def).expect_err(
        "dispatch must fail when input_schema requires fields but zero bindings exist",
    );
    let message = error.to_string();
    assert!(
        message.contains("foo") || message.contains("bar"),
        "error should name at least one missing required field, got {message}"
    );
}
