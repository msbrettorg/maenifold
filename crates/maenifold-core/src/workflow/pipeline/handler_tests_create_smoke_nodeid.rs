use super::handler::pipeline;
use super::test_support::*;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

fn payload(node: Value) -> Value {
    json!({ "definition": { "nodes": [node], "edges": [] } })
}

fn create_response(context: &ToolContext, payload: Value) -> Value {
    serde_json::from_str(&pipeline(context, &payload).expect("create pipeline"))
        .expect("parse response")
}

fn create_error(context: &ToolContext, payload: Value) -> String {
    pipeline(context, &payload)
        .expect_err("expected create error")
        .to_string()
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_nid01_nid02_nid03_accepts_common_node_id_formats() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    for node_id in ["alpha1", "node-1_alpha", "release.v1.2"] {
        let response = create_response(
            &context,
            payload(
                json!({ "id": node_id, "workflowId": "test-workflow-a", "acpClient": "agent-node" }),
            ),
        );
        let node = response["data"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.first())
            .expect("node");

        assert_eq!(response["data"]["ready"], json!([node_id]));
        assert_eq!(node["nodeId"], node_id);
        assert!(
            node["sessionId"]
                .as_str()
                .is_some_and(|id| id.ends_with(&format!("-{node_id}")))
        );
    }
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_nid06_accepts_node_id_with_spaces() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let node_id = "node with spaces";
    let response = create_response(
        &context,
        payload(
            json!({ "id": node_id, "workflowId": "test-workflow-a", "acpClient": "agent-node" }),
        ),
    );

    assert_eq!(response["data"]["ready"], json!([node_id]));
    assert_eq!(response["data"]["nodes"][0]["nodeId"], node_id);
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_nid08_rejects_absent_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(json!({ "workflowId": "test-workflow-a", "acpClient": "agent-node" })),
    );

    assert!(
        error.contains("missing field") && error.contains("id"),
        "unexpected error: {error}"
    );
}

// BUG: empty node IDs are accepted and generate malformed workflow session IDs
#[test]
#[ignore]
#[serial(config)]
fn pipeline_create_smoke_nid04_rejects_empty_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(json!({ "id": "", "workflowId": "test-workflow-a", "acpClient": "agent-node" })),
    );
    assert!(error.contains("id"), "unexpected error: {error}");
}

// BUG: very long node IDs are not validated before filesystem-backed session creation
#[test]
#[ignore]
#[serial(config)]
fn pipeline_create_smoke_nid05_rejects_very_long_node_id_gracefully() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let node_id = "n".repeat(1000);
    let error = create_error(
        &context,
        payload(
            json!({ "id": node_id, "workflowId": "test-workflow-a", "acpClient": "agent-node" }),
        ),
    );

    assert!(
        error.contains("node") && error.contains("id"),
        "unexpected error: {error}"
    );
}
