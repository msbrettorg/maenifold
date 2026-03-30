use super::handler::pipeline;
use super::test_support::*;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serial_test::serial;

fn view_node(node: Value) -> Value {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let create_output = pipeline(
        &context,
        &json!({ "definition": { "nodes": [node], "edges": [] } }),
    )
    .expect("create pipeline");
    let create_response: Value =
        serde_json::from_str(&create_output).expect("parse create response");
    let pipeline_id = create_response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");
    let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view pipeline");
    let view_response: Value = serde_json::from_str(&view_output).expect("parse view response");
    view_response["data"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.first())
        .cloned()
        .expect("viewed node")
}

fn create_error(node: Value) -> String {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    pipeline(
        &context,
        &json!({ "definition": { "nodes": [node], "edges": [] } }),
    )
    .expect_err("pipeline creation should fail")
    .to_string()
}

fn assert_error_contains(node: Value, parts: &[&str]) {
    let message = create_error(node);
    for part in parts {
        assert!(
            message.contains(part),
            "missing '{part}' in error: {message}"
        );
    }
}

#[test]
#[serial(config)]
fn test_smoke_nf01_workflow_valid_id() {
    assert_eq!(
        view_node(json!({ "id": "wf-valid", "workflowId": "test-workflow-a" }))["workflowId"],
        "test-workflow-a"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf02_workflow_nonexistent_id() {
    assert_error_contains(
        json!({ "id": "wf-missing", "workflowId": "missing-workflow" }),
        &["Workflow 'missing-workflow' not found"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf03_workflow_empty_id() {
    assert_error_contains(
        json!({ "id": "wf-empty", "workflowId": "" }),
        &["wf-empty", "workflowId", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf04_workflow_acp_client_present() {
    assert_eq!(
        view_node(
            json!({ "id": "wf-agent", "workflowId": "test-workflow-a", "acpClient": "workflow-agent" })
        )["agent"],
        "workflow-agent"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf05_workflow_acp_client_absent_defaults_null() {
    assert_eq!(
        view_node(json!({ "id": "wf-no-agent", "workflowId": "test-workflow-a" }))["agent"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf06_workflow_prompt_present() {
    assert_eq!(
        view_node(
            json!({ "id": "wf-prompt", "workflowId": "test-workflow-a", "prompt": "Summarize upstream output" })
        )["prompt"],
        "Summarize upstream output"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf07_workflow_prompt_absent_defaults_null() {
    assert_eq!(
        view_node(json!({ "id": "wf-no-prompt", "workflowId": "test-workflow-a" }))["prompt"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf08_workflow_timeout_present_positive() {
    assert_eq!(
        view_node(json!({ "id": "wf-timeout", "workflowId": "test-workflow-a", "timeout": 30 }))
            ["timeout"],
        30
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf09_workflow_timeout_absent_defaults_null() {
    assert_eq!(
        view_node(json!({ "id": "wf-timeout-null", "workflowId": "test-workflow-a" }))["timeout"],
        Value::Null
    );
}

#[test]
#[ignore]
// BUG: Pipeline accepts workflow timeout=0 instead of rejecting non-positive timeout values.
#[serial(config)]
fn test_smoke_nf10_workflow_timeout_zero() {
    let message = create_error(json!({
        "id": "wf-timeout-zero",
        "workflowId": "test-workflow-a",
        "timeout": 0
    }));
    assert!(
        message.contains("timeout")
            && (message.contains("positive") || message.contains("greater than 0")),
        "unexpected error: {message}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf11_workflow_timeout_negative() {
    assert_error_contains(
        json!({ "id": "wf-timeout-neg", "workflowId": "test-workflow-a", "timeout": -1 }),
        &["JSON error", "u64"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf12_prompt_valid_string() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-valid",
            "type": "prompt",
            "prompt": "Draft the operator summary",
            "agent": "prompt-agent"
        }))["prompt"],
        "Draft the operator summary"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf13_prompt_empty_string() {
    assert_error_contains(
        json!({ "id": "prompt-empty", "type": "prompt", "prompt": "", "agent": "prompt-agent" }),
        &["prompt-empty", "prompt", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf14_prompt_agent_valid_string() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent",
            "type": "prompt",
            "prompt": "Draft the operator summary",
            "agent": "writer-agent"
        }))["agent"],
        "writer-agent"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf15_prompt_agent_empty_string() {
    assert_error_contains(
        json!({ "id": "prompt-no-agent", "type": "prompt", "prompt": "Hello", "agent": "" }),
        &["prompt-no-agent", "agent", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf16_prompt_tool_mode_disabled() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-disabled",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "toolMode": "disabled"
        }))["toolMode"],
        "disabled"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf17_prompt_tool_mode_auto() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-auto",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "toolMode": "auto"
        }))["toolMode"],
        "auto"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf18_prompt_tool_mode_absent_defaults_disabled() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-default-mode",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent"
        }))["toolMode"],
        "disabled"
    );
}
