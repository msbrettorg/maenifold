use super::workflow;
use crate::config::Config;
use crate::error::MaenifoldError;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::fs;

fn context_with_workflow(
    workflow_id: &str,
    inputs_decl: Option<Value>,
) -> (tempfile::TempDir, ToolContext) {
    let dir = tempfile::tempdir().expect("tempdir");
    let workflows_dir = dir.path().join("assets").join("workflows");
    fs::create_dir_all(&workflows_dir).expect("workflows dir");

    let mut asset = json!({
        "id": workflow_id,
        "name": workflow_id,
        "steps": [
            {
                "id": "first",
                "name": "First",
                "description": "First step"
            }
        ]
    });
    if let Some(inputs_decl) = inputs_decl {
        asset["inputs"] = inputs_decl;
    }

    fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_string_pretty(&asset).expect("asset json"),
    )
    .expect("write workflow asset");

    Config::override_root(dir.path());
    let config = Config::current();
    Config::reset_overrides();
    (dir, ToolContext::new(config, OutputFormat::Json))
}

fn workflow_error(ctx: &ToolContext, payload: Value) -> String {
    match workflow(ctx, &payload) {
        Err(MaenifoldError::InvalidArgument(message)) => message,
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
#[serial]
fn unknown_input_key_is_rejected() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "target": { "type": "string" } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": { "unknown": "x" } }),
    );

    assert!(message.contains("Unknown input key 'unknown'"));
    assert!(message.contains("input-contract"));
}

#[test]
#[serial]
fn missing_required_input_key_is_rejected() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "target": { "type": "string", "required": true } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": {} }),
    );

    assert!(message.contains("Missing required input key 'target'"));
    assert!(message.contains("input-contract"));
}

#[test]
#[serial]
fn missing_inputs_object_rejects_missing_required_key() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "target": { "type": "string", "required": true } })),
    );

    let message = workflow_error(&ctx, json!({ "workflowId": "input-contract" }));

    assert!(message.contains("Missing required input key 'target'"));
}

#[test]
#[serial]
fn string_input_type_mismatch_is_rejected() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "target": { "type": "string" } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": { "target": 42 } }),
    );

    assert!(message.contains("target"));
    assert!(message.contains("string"));
}

#[test]
#[serial]
fn number_input_type_mismatch_is_rejected() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "count": { "type": "number" } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": { "count": "42" } }),
    );

    assert!(message.contains("count"));
    assert!(message.contains("number"));
}

#[test]
#[serial]
fn boolean_input_type_mismatch_is_rejected() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "flag": { "type": "boolean" } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": { "flag": "true" } }),
    );

    assert!(message.contains("flag"));
    assert!(message.contains("boolean"));
}

#[test]
#[serial]
fn select_input_rejects_values_outside_options() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "mode": { "type": "select", "options": ["a", "b"] } })),
    );

    let message = workflow_error(
        &ctx,
        json!({ "workflowId": "input-contract", "inputs": { "mode": "c" } }),
    );

    assert!(message.contains("mode"));
    assert!(message.contains("select"));
}

#[test]
#[serial]
fn select_input_accepts_declared_option() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "mode": { "type": "select", "options": ["a", "b"] } })),
    );

    let result = workflow(
        &ctx,
        &json!({ "workflowId": "input-contract", "inputs": { "mode": "a" } }),
    );

    assert!(result.is_ok(), "valid select input should start workflow");
}

#[test]
#[serial]
fn valid_string_input_starts_workflow() {
    let (_dir, ctx) = context_with_workflow(
        "input-contract",
        Some(json!({ "target": { "type": "string" } })),
    );

    let result = workflow(
        &ctx,
        &json!({ "workflowId": "input-contract", "inputs": { "target": "valid" } }),
    );

    assert!(result.is_ok(), "valid input should not fail validation");
}

#[test]
#[serial]
fn undeclared_inputs_block_preserves_existing_permissive_behavior() {
    let (_dir, ctx) = context_with_workflow("input-contract", None);

    let result = workflow(
        &ctx,
        &json!({ "workflowId": "input-contract", "inputs": { "extra": "allowed" } }),
    );

    assert!(
        result.is_ok(),
        "workflows without an inputs declaration remain schema-free"
    );
}
