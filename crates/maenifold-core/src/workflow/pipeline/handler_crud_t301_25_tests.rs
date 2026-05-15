use serde_json::{Value, json};
use serial_test::serial;

use crate::util::json::OutputFormat;

use super::handler::pipeline;
use super::session_io::read_pipeline_session;
use super::test_support::pipeline_test_context;

fn single_tool_definition() -> Value {
    json!({
        "nodes": [
            { "id": "tool", "type": "tool", "toolName": "GetConfig" }
        ],
        "edges": []
    })
}

fn pipeline_id_from(response: &str) -> String {
    let payload: Value = serde_json::from_str(response).expect("json response");
    payload["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string()
}

#[test]
#[serial]
fn create_pipeline_t301_25_persists_name_and_description_frontmatter() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let response = pipeline(
        &context,
        &json!({
            "definition": single_tool_definition(),
            "name": "my-pipeline",
            "description": "test desc"
        }),
    )
    .expect("create pipeline");
    let pipeline_id = pipeline_id_from(&response);
    let (_, frontmatter, _) = read_pipeline_session(&context, &pipeline_id).expect("read pipeline");

    assert_eq!(
        frontmatter.get("name").and_then(serde_yml::Value::as_str),
        Some("my-pipeline")
    );
    assert_eq!(
        frontmatter.get("description").and_then(serde_yml::Value::as_str),
        Some("test desc")
    );
}

#[test]
#[serial]
fn create_pipeline_t301_25_omits_blank_name_and_description_frontmatter() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let response = pipeline(
        &context,
        &json!({
            "definition": single_tool_definition()
        }),
    )
    .expect("create pipeline");
    let pipeline_id = pipeline_id_from(&response);
    let (_, frontmatter, _) = read_pipeline_session(&context, &pipeline_id).expect("read pipeline");

    assert!(
        !frontmatter.contains_key("name"),
        "name should be absent when create payload omits it"
    );
    assert!(
        !frontmatter.contains_key("description"),
        "description should be absent when create payload omits it"
    );
}

#[test]
#[serial]
fn pipeline_t301_25_view_response_includes_persisted_name() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let response = pipeline(
        &context,
        &json!({
            "definition": single_tool_definition(),
            "name": "my-pipeline",
            "description": "test desc"
        }),
    )
    .expect("create pipeline");
    let pipeline_id = pipeline_id_from(&response);

    let view_response = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view pipeline");
    let payload: Value = serde_json::from_str(&view_response).expect("json response");

    assert_eq!(payload["data"]["name"], "my-pipeline");
}
