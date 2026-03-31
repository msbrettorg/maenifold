use super::handler::pipeline;
use super::test_support::pipeline_test_context;
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use serial_test::serial;

// NOTE: duplicated across param_smoke files for test isolation
fn solo_definition() -> Value {
    json!({ "nodes": [{ "id": "solo", "workflowId": "test-workflow-a" }], "edges": [] })
}

fn create_pipeline_id(context: &ToolContext) -> String {
    let output =
        pipeline(context, &json!({ "definition": solo_definition() })).expect("create pipeline");
    serde_json::from_str::<Value>(&output).expect("parse response")["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string()
}

// NOTE: duplicated across param_smoke files for test isolation
fn assert_invalid_combo(error: impl std::fmt::Display) {
    assert!(
        error.to_string().contains("Invalid parameter combination"),
        "unexpected error: {error}"
    );
}

fn seed_workflow_session(context: &ToolContext, session_id: &str) {
    let workflow_id = "test-workflow-a".to_string();
    let frontmatter = crate::workflow::initial_workflow_frontmatter(
        session_id,
        std::slice::from_ref(&workflow_id),
        "pending",
    );
    write_markdown(
        &workflow_session_uri(session_id),
        Some(&frontmatter),
        "# Workflow Session\n",
        &context.config,
    )
    .expect("write workflow session");
}

#[test]
#[serial(config)]
fn test_smoke_pi01_definition_alone() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert!(pipeline(&context, &json!({ "definition": solo_definition() })).is_ok());
}

// BUG: definition + sessionId should be rejected as a conflicting create-vs-session operation.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_pi02_definition_with_session_id_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({
            "sessionId": "pipeline-1735689600001", "definition": solo_definition()
        }),
    )
    .expect_err("definition + sessionId should conflict");
    assert!(error.to_string().to_lowercase().contains("conflict"));
}

#[test]
#[serial(config)]
fn test_smoke_pi03_definition_as_string() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error =
        pipeline(&context, &json!({ "definition": "nope" })).expect_err("string definition");
    assert!(
        error.to_string().contains("invalid type"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi04_definition_as_null() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "definition": null })).expect_err("null definition");
    assert!(
        error.to_string().contains("invalid type"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi05_definition_with_empty_nodes_and_edges() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "definition": { "nodes": [], "edges": [] } }),
    )
    .expect_err("empty definition should fail");
    assert!(
        error.to_string().contains("at least one node"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi06_session_id_alone() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "sessionId": "pipeline-1735689600002" }))
            .expect_err("session only"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi07_session_id_as_number() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "sessionId": 7 })).expect_err("numeric session id"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi08_session_id_as_empty_string() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "sessionId": "" })).expect_err("empty session id"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi09_session_id_pointing_to_workflow_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let session_id = "workflow-1735689600003-root";
    seed_workflow_session(&context, session_id);
    let error = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
        .expect_err("workflow session should not view as pipeline");
    assert!(
        error.to_string().contains("doesn't exist"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi10_max_concurrency_alone() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "maxConcurrency": 3 })).expect_err("maxConcurrency alone"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi11_max_concurrency_as_float() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "definition": solo_definition(), "maxConcurrency": 1.5 }),
    )
    .expect_err("float maxConcurrency");
    assert!(
        error
            .to_string()
            .contains("maxConcurrency must be a positive integer"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi12_max_concurrency_very_large() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let output = pipeline(
        &context,
        &json!({ "definition": solo_definition(), "maxConcurrency": 1000 }),
    )
    .expect("large maxConcurrency should succeed");
    let parsed: Value = serde_json::from_str(&output).expect("parse");
    let pipeline_id = parsed["data"]["pipelineId"].as_str().expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    assert_eq!(
        frontmatter
            .expect("frontmatter")
            .get("maxConcurrency")
            .and_then(YamlValue::as_u64),
        Some(1000)
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi13_max_concurrency_as_null() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "definition": solo_definition(), "maxConcurrency": null }),
    )
    .expect_err("null maxConcurrency");
    assert!(
        error
            .to_string()
            .contains("maxConcurrency must be a positive integer"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi14_view_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "view": true })).expect_err("view without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi15_view_as_string_true() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = create_pipeline_id(&context);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "view": "true" }),
        )
        .expect_err("string view should not route"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi16_dispatch_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "dispatch": true })).expect_err("dispatch without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi17_list_alone() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let output = pipeline(&context, &json!({ "list": true })).expect("list should succeed");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelines"],
        json!([])
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi18_list_with_session_id_extra_param() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let output = pipeline(
        &context,
        &json!({ "list": true, "sessionId": "pipeline-1735689600004" }),
    )
    .expect("list should ignore extra sessionId");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelines"],
        json!([])
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi19_run_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "run": true })).expect_err("run without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi20_run_with_prompt_but_no_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "run": true, "prompt": "ambient" }))
            .expect_err("run+prompt without session"),
    );
}
