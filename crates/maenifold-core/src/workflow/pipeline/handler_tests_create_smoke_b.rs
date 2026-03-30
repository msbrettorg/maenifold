use super::handler::pipeline;
use super::test_support::*;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serial_test::serial;

fn payload(nodes: Vec<Value>, edges: Vec<Value>) -> Value {
    json!({ "definition": { "nodes": nodes, "edges": edges } })
}

fn payload_with_max(nodes: Vec<Value>, edges: Vec<Value>, max_concurrency: Value) -> Value {
    json!({
        "definition": { "nodes": nodes, "edges": edges },
        "maxConcurrency": max_concurrency
    })
}

fn workflow_node(id: &str, workflow_id: &str) -> Value {
    json!({ "id": id, "workflowId": workflow_id })
}

fn create_error(context: &ToolContext, payload: Value) -> String {
    pipeline(context, &payload)
        .expect_err("expected create error")
        .to_string()
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c09_rejects_empty_node_list() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(&context, payload(vec![], vec![]));
    assert!(
        error.contains("must contain at least one node"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c10_rejects_duplicate_node_ids() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(
            vec![
                workflow_node("dup", "test-workflow-a"),
                workflow_node("dup", "test-workflow-b"),
            ],
            vec![],
        ),
    );
    assert!(
        error.contains("duplicate node ID 'dup'"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c12_c13_rejects_unknown_edge_endpoints() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    for (edge, expected) in [
        (
            json!({ "source": "missing", "target": "a" }),
            "unknown source node 'missing'",
        ),
        (
            json!({ "source": "a", "target": "missing" }),
            "unknown target node 'missing'",
        ),
    ] {
        let error = create_error(
            &context,
            payload(vec![workflow_node("a", "test-workflow-a")], vec![edge]),
        );
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c16_rejects_duplicate_edge_pair() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(
            vec![
                workflow_node("a", "test-workflow-a"),
                workflow_node("b", "test-workflow-b"),
            ],
            vec![
                json!({ "source": "a", "target": "b" }),
                json!({ "source": "a", "target": "b", "type": "always" }),
            ],
        ),
    );

    assert!(
        error.contains("duplicate edge 'a -> b'"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c18_rejects_prompt_node_with_empty_agent() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(
            vec![json!({
                "id": "prompt",
                "type": "prompt",
                "prompt": "Summarize",
                "agent": "   "
            })],
            vec![],
        ),
    );

    assert!(
        error.contains("prompt") && error.contains("agent"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c20_rejects_zero_total_thoughts() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(
            vec![json!({
                "id": "thinking",
                "type": "sequentialThinking",
                "topic": "Review output",
                "totalThoughts": 0,
                "agent": "thinking-agent"
            })],
            vec![],
        ),
    );

    assert!(
        error.contains("thinking") && error.contains("totalThoughts"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c21_rejects_unknown_node_type() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(vec![json!({ "id": "mystery", "type": "badType" })], vec![]),
    );

    assert!(
        error.contains("mystery") && error.contains("unknown type") && error.contains("badType"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c22_rejects_node_without_type_or_workflow_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(&context, payload(vec![json!({ "id": "n99" })], vec![]));
    assert!(
        error.contains("n99") && error.contains("missing 'type' field"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c24_rejects_string_max_concurrency() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload_with_max(
            vec![workflow_node("solo", "test-workflow-a")],
            vec![],
            json!("2"),
        ),
    );
    assert!(
        error.contains("maxConcurrency must be a positive integer"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c25_rejects_malformed_definition_json() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        json!({ "definition": { "nodes": "bad", "edges": [] } }),
    );
    assert!(error.contains("invalid type"), "unexpected error: {error}");
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c26_rejects_self_referencing_edge() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = create_error(
        &context,
        payload(
            vec![workflow_node("self", "test-workflow-a")],
            vec![json!({ "source": "self", "target": "self" })],
        ),
    );
    assert!(
        error.contains("contains a cycle"),
        "unexpected error: {error}"
    );
}
