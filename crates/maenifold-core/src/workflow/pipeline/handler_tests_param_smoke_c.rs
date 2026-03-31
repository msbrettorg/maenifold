use super::handler::pipeline;
use super::supervisor_test_support::reset_supervisor;
use super::test_support::{pipeline_test_context, seed_dispatch_pipeline};
use super::{NodeState, NodeStatus};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

// NOTE: duplicated across param_smoke files for test isolation
fn solo_definition() -> Value {
    json!({ "nodes": [{ "id": "solo", "workflowId": "test-workflow-a" }], "edges": [] })
}

// NOTE: duplicated across param_smoke files for test isolation
fn completed_states(prefix: &str) -> HashMap<String, NodeState> {
    HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-root"),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("done".to_string()),
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-child"),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("done".to_string()),
            },
        ),
    ])
}

// NOTE: duplicated across param_smoke files for test isolation
fn active_states(prefix: &str) -> HashMap<String, NodeState> {
    HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-root"),
                status: NodeStatus::Active,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-child"),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ])
}

fn abandoned_states(prefix: &str) -> HashMap<String, NodeState> {
    HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-root"),
                status: NodeStatus::Abandoned,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-child"),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ])
}

// Used by #[ignore]d conflict detection tests (px01-px14) — will become active when BUG: no-conflict-detection is fixed
#[allow(dead_code)]
fn assert_conflict(error: impl std::fmt::Display) {
    let text = error.to_string().to_lowercase();
    assert!(text.contains("conflict"), "unexpected error: {error}");
}

// BUG: handler does not reject run + pause as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px01_run_and_pause_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600201", "run": true, "pause": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject run + cancel as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px02_run_and_cancel_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600202", "run": true, "cancel": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject run + dispatch as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px03_run_and_dispatch_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600203", "run": true, "dispatch": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject pause + resume as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px04_pause_and_resume_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600204", "pause": true, "resume": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject pause + cancel as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px05_pause_and_cancel_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600205", "pause": true, "cancel": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject graft + join as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px06_graft_and_join_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600206", "graft": { "nodes": [], "edges": [] }, "join": { "sourceSessionId": "pipeline-1735689600207", "edges": [] } })).expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject graft + fork as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px07_graft_and_fork_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600208", "graft": { "nodes": [], "edges": [] }, "fork": { "fromNodeId": "root" } })).expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject delete + view as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px08_delete_and_view_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600209", "delete": true, "view": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject dispatch + delete as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px09_dispatch_and_delete_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600210", "dispatch": true, "delete": true }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject status + restart as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px10_status_and_restart_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600211", "nodeId": "root", "status": "completed", "conclusion": "done", "restart": true })).expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject definition + delete as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px11_definition_and_delete_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600212", "definition": solo_definition(), "delete": true })).expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject rewind + graft as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px12_rewind_and_graft_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600213", "rewind": { "nodeIds": ["root"] }, "graft": { "nodes": [], "edges": [] } })).expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject list + definition as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px13_list_and_definition_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "list": true, "definition": solo_definition() }),
    )
    .expect_err("conflict");
    assert_conflict(error);
}

// BUG: handler does not reject learn + operation as conflicting operations.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_px14_learn_and_operation_conflict() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(&context, &json!({ "learn": true, "list": true })).expect_err("conflict");
    assert_conflict(error);
}

#[test]
#[serial(config)]
fn test_smoke_px15_run_and_prompt_valid_combo() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _supervisor = reset_supervisor();
    let output = pipeline(&context, &json!({ "definition": solo_definition() })).expect("create");
    let pipeline_id = serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string();
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "run": true, "prompt": "  ambient  " }),
    )
    .expect("run");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["status"],
        "started"
    );
}

#[test]
#[serial(config)]
fn test_smoke_px16_status_conclusion_and_node_id_valid_combo() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600216";
    seed_dispatch_pipeline(&context, pipeline_id, active_states("1735689600216"), 2);
    let output = pipeline(&context, &json!({ "sessionId": pipeline_id, "nodeId": "root", "status": "completed", "conclusion": "done" })).expect("status update");
    let parsed: Value = serde_json::from_str(&output).expect("parse");
    let root = parsed["data"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["nodeId"] == "root")
        .expect("root node");
    assert_eq!(root["status"], "completed");
}

#[test]
#[serial(config)]
fn test_smoke_px17_definition_and_max_concurrency_valid_combo() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let output = pipeline(
        &context,
        &json!({ "definition": solo_definition(), "maxConcurrency": 3 }),
    )
    .expect("create");
    let parsed: Value = serde_json::from_str(&output).expect("parse");
    let pipeline_id = parsed["data"]["pipelineId"].as_str().expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read");
    assert_eq!(
        frontmatter
            .expect("frontmatter")
            .get("maxConcurrency")
            .and_then(YamlValue::as_u64),
        Some(3)
    );
}

#[test]
#[serial(config)]
fn test_smoke_px18_fork_and_new_session_id_valid_combo() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600218";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600218"), 2);
    let output = pipeline(&context, &json!({ "sessionId": pipeline_id, "fork": { "fromNodeId": "root", "newSessionId": "pipeline-forked-smoke" } })).expect("fork");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelineId"],
        "pipeline-forked-smoke"
    );
}

#[test]
#[serial(config)]
fn test_smoke_px19_restart_and_node_id_valid_combo() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600219";
    seed_dispatch_pipeline(&context, pipeline_id, abandoned_states("1735689600219"), 2);
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "nodeId": "root", "restart": true }),
    )
    .expect("restart");
    let parsed: Value = serde_json::from_str(&output).expect("parse");
    let root = parsed["data"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["nodeId"] == "root")
        .expect("root node");
    assert_eq!(root["status"], "pending");
}
