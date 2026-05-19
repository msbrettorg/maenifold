use std::collections::HashMap;

use serde_json::json;

use crate::error::MaenifoldError;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::validate_pipeline_inputs;

use super::executor_helpers::DispatchedNode;
use super::executor_nodes::build_node_execution_request;
use super::handler_lifecycle::handle_lifecycle_operation;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{
    NodeState, NodeStatus, PipelineDefinition, PipelineNode, validate_pipeline,
};

fn required_project_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "required": ["project_name"],
        "properties": {
            "project_name": { "type": "string" }
        }
    })
}

fn tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: "test.tool".to_string(),
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

fn pending_node_state(session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status: NodeStatus::Pending,
        attempts: 0,
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

fn definition_with_schema(
    input_schema: Option<serde_json::Value>,
    inputs: Option<serde_json::Map<String, serde_json::Value>>,
) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![tool_node("tool")],
        edges: vec![],
        inputs,
        input_schema,
        template_id: None,
    }
}

fn assert_invalid_argument_names(error: MaenifoldError, expected: &[&str]) {
    match error {
        MaenifoldError::InvalidArgument(message) => {
            for part in expected {
                assert!(
                    message.contains(part),
                    "expected error message {message:?} to contain {part:?}"
                );
            }
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn input_schema_validates_at_definition_save() {
    let def = definition_with_schema(Some(required_project_schema()), Some(serde_json::Map::new()));

    let error = validate_pipeline(&def).expect_err("missing project_name must reject definition");

    assert_invalid_argument_names(error, &["project_name"]);
}

#[test]
fn input_schema_validates_at_run_start() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000003000-input-schema";
    let definition = definition_with_schema(Some(required_project_schema()), None);
    let node_states = HashMap::from([(
        "tool".to_string(),
        pending_node_state("workflow-1700000003000-tool"),
    )]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    let result = handle_lifecycle_operation(
        &context,
        session_id,
        &json!({ "run": true, "prompt": "go", "inputs": {} }),
    );

    assert_invalid_argument_names(
        result.expect_err("run start must validate before driver spawn"),
        &["project_name"],
    );
}

#[test]
fn input_schema_validates_at_node_dispatch() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = definition_with_schema(Some(required_project_schema()), None);
    let inputs = serde_json::Map::new();
    let node_states = HashMap::from([(
        "tool".to_string(),
        pending_node_state("workflow-1700000003001-tool"),
    )]);
    let dispatch = DispatchedNode {
        node_id: "tool".to_string(),
        session_id: Some("workflow-1700000003001-tool".to_string()),
    };
    let node = definition.nodes.first().expect("tool node");

    let error = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        node,
        &dispatch,
        "go",
        &inputs,
    )
    .expect_err("node dispatch must revalidate pipeline inputs");

    assert_invalid_argument_names(error, &["project_name"]);
}

#[test]
fn missing_required_input_errors_with_field_name() {
    let schema = json!({
        "type": "object",
        "required": ["alpha", "beta"],
        "properties": {
            "alpha": { "type": "string" },
            "beta": { "type": "string" }
        }
    });
    let inputs = serde_json::Map::from_iter([("alpha".to_string(), json!("present"))]);

    let error =
        validate_pipeline_inputs(&schema, &inputs).expect_err("missing beta must be rejected");

    assert_invalid_argument_names(error, &["beta"]);
}

#[test]
fn wrong_type_input_errors_with_path() {
    let schema = json!({
        "type": "object",
        "properties": { "count": { "type": "integer" } }
    });
    let inputs = serde_json::Map::from_iter([("count".to_string(), json!("not an int"))]);

    let error =
        validate_pipeline_inputs(&schema, &inputs).expect_err("wrong count type must be rejected");

    assert_invalid_argument_names(error, &["count", "type"]);
}

#[test]
fn additional_properties_false_rejects_extra_keys() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": { "known": { "type": "string" } }
    });
    let inputs = serde_json::Map::from_iter([
        ("known".to_string(), json!("ok")),
        ("surprise".to_string(), json!(true)),
    ]);

    let error = validate_pipeline_inputs(&schema, &inputs).expect_err("extra key must be rejected");

    assert_invalid_argument_names(error, &["surprise"]);
}

#[test]
fn no_input_schema_means_no_validation() {
    let def = definition_with_schema(
        None,
        Some(serde_json::Map::from_iter([(
            "anything".to_string(),
            json!({ "nested": true }),
        )])),
    );

    validate_pipeline(&def).expect("pipelines without input_schema keep accepting arbitrary inputs");
}
