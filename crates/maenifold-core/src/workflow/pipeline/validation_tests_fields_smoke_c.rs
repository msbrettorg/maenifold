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

fn assert_view_field(node: Value, field: &str, expected: Value) {
    assert_eq!(view_node(node)[field], expected);
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
fn test_smoke_nf36_thinking_topic_valid_string() {
    assert_view_field(
        json!({ "id": "think-topic", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent" }),
        "topic",
        json!("Review execution path"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf37_thinking_topic_empty_string() {
    assert_error_contains(
        json!({ "id": "think-topic-empty", "type": "sequentialThinking", "topic": "", "agent": "think-agent" }),
        &["think-topic-empty", "topic", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf38_thinking_agent_valid_string() {
    assert_view_field(
        json!({ "id": "think-agent", "type": "sequentialThinking", "topic": "Review execution path", "agent": "reasoner" }),
        "agent",
        json!("reasoner"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf39_thinking_agent_empty_string() {
    assert_error_contains(
        json!({ "id": "think-agent-empty", "type": "sequentialThinking", "topic": "Review execution path", "agent": "" }),
        &["think-agent-empty", "agent", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf40_thinking_total_thoughts_minimum() {
    assert_view_field(
        json!({ "id": "think-min", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "totalThoughts": 1 }),
        "totalThoughts",
        json!(1),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf41_thinking_total_thoughts_large() {
    assert_view_field(
        json!({ "id": "think-large", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "totalThoughts": 100 }),
        "totalThoughts",
        json!(100),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf42_thinking_total_thoughts_zero() {
    assert_error_contains(
        json!({ "id": "think-zero", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "totalThoughts": 0 }),
        &["think-zero", "totalThoughts", "greater than 0"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf43_thinking_total_thoughts_negative() {
    assert_error_contains(
        json!({ "id": "think-negative", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "totalThoughts": -1 }),
        &["JSON error", "u32"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf44_thinking_total_thoughts_absent_defaults_five() {
    assert_view_field(
        json!({ "id": "think-default", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent" }),
        "totalThoughts",
        json!(5),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf45_thinking_analysis_type_present() {
    assert_view_field(
        json!({ "id": "think-analysis", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "analysisType": "diagnostic" }),
        "analysisType",
        json!("diagnostic"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf46_thinking_analysis_type_absent_defaults_null() {
    assert_view_field(
        json!({ "id": "think-analysis-null", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent" }),
        "analysisType",
        Value::Null,
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf47_tool_name_valid_string() {
    assert_view_field(
        json!({ "id": "tool-name", "type": "tool", "toolName": "ReadMemory" }),
        "toolName",
        json!("ReadMemory"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf48_tool_name_empty_string() {
    assert_error_contains(
        json!({ "id": "tool-name-empty", "type": "tool", "toolName": "" }),
        &["tool-name-empty", "toolName", "must not be empty"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf49_tool_args_valid_object() {
    assert_view_field(
        json!({ "id": "tool-args", "type": "tool", "toolName": "ReadMemory", "toolArgs": { "identifier": "memory://build/test" } }),
        "toolArgs",
        json!({ "identifier": "memory://build/test" }),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf50_tool_args_empty_object() {
    assert_view_field(
        json!({ "id": "tool-args-empty", "type": "tool", "toolName": "ReadMemory", "toolArgs": {} }),
        "toolArgs",
        json!({}),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf51_tool_args_absent_defaults_null() {
    assert_view_field(
        json!({ "id": "tool-args-null", "type": "tool", "toolName": "ReadMemory" }),
        "toolArgs",
        Value::Null,
    );
}

#[test]
#[ignore]
// BUG: Pipeline accepts non-object toolArgs values even though the MCP schema advertises toolArgs as an object.
#[serial(config)]
fn test_smoke_nf52_tool_args_string_instead_of_object() {
    assert_error_contains(
        json!({ "id": "tool-args-string", "type": "tool", "toolName": "ReadMemory", "toolArgs": "memory://build/test" }),
        &["toolArgs", "object"],
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf53_tool_agent_present() {
    assert_view_field(
        json!({ "id": "tool-agent", "type": "tool", "toolName": "ReadMemory", "agent": "tool-agent" }),
        "agent",
        json!("tool-agent"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf54_tool_agent_absent_defaults_null() {
    assert_view_field(
        json!({ "id": "tool-agent-null", "type": "tool", "toolName": "ReadMemory" }),
        "agent",
        Value::Null,
    );
}
