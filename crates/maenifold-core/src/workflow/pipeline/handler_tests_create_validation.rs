use super::handler::pipeline;
use super::test_support::pipeline_test_context;
use crate::util::json::OutputFormat;
use serde_json::json;
use serial_test::serial;

#[test]
#[serial(config)]
fn pipeline_create_rejects_workflow_nodes_missing_acp_client() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({
            "definition": {
                "nodes": [{ "id": "root", "workflowId": "test-workflow-a" }],
                "edges": []
            }
        }),
    )
    .expect_err("missing acpClient should fail");

    let message = error.to_string();
    assert!(message.contains("root"), "unexpected error: {message}");
    assert!(message.contains("acpClient"), "unexpected error: {message}");
}

#[test]
#[serial(config)]
fn pipeline_create_rejects_prompt_and_thinking_nodes_missing_agent() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    for node in [
        json!({ "id": "prompt", "type": "prompt", "prompt": "Summarize" }),
        json!({
            "id": "thinking",
            "type": "sequentialThinking",
            "topic": "Review output",
            "totalThoughts": 2
        }),
    ] {
        let error = pipeline(
            &context,
            &json!({ "definition": { "nodes": [node], "edges": [] } }),
        )
        .expect_err("missing agent should fail");
        let message = error.to_string();
        assert!(message.contains("agent"), "unexpected error: {message}");
    }
}
