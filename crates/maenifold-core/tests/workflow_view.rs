mod workflow_support;

use serde_json::json;
use workflow_support::{
    TEST_MUTEX, invoke_workflow, start_session, workflow_operations_env, workflow_tools_env,
};

// T-12.0: RTM NFR-1 - isolate workflow queue view assertions into a small, readable test module.
#[test]
fn view_queue_shows_current_position_and_next_step() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let queue_json =
        serde_json::to_string(&["wtt-alpha", "wtt-beta", "wtt-gamma"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view workflow queue");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(output.contains("wtt-alpha"), "unexpected output: {output}");
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(output.contains("wtt-gamma"), "unexpected output: {output}");
    assert!(output.contains("Position:"), "unexpected output: {output}");
    assert!(output.contains("step 1/3"), "unexpected output: {output}");
}

#[test]
fn view_reflects_updated_position_after_step_advancement() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[step-tracking]]."
    }))
    .expect("advance workflow");
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view advanced workflow");
    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");
}

#[test]
fn view_displays_queue_status_with_current_position() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2", "test-workflow-3"])
            .expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let output =
        invoke_workflow(json!({ "sessionId": session_id, "view": true })).expect("view queue");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(
        output.contains("test-workflow-1"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-2"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-3"),
        "unexpected output: {output}"
    );
    assert!(output.contains("Position:"), "unexpected output: {output}");
    assert!(
        output.contains("workflow 1/3"),
        "unexpected output: {output}"
    );
    assert!(output.contains("step 1/3"), "unexpected output: {output}");
}

#[test]
fn view_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let invalid_session_id = "workflow-nonexistent-12345";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "view": true
    }))
    .expect("invalid view output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains(invalid_session_id),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("doesn't exist"),
        "unexpected output: {output}"
    );
}

#[test]
fn view_after_progressing_shows_updated_position() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[view-state]]."
    }))
    .expect("advance workflow for view");
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view updated workflow");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");
}

#[test]
fn view_via_workflow_tools_entry_point() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view via workflow tool");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(output.contains("Position:"), "unexpected output: {output}");
}

// T-15.14: RTM FR-8 — view-without-session-id must return error, not success with help text.
#[test]
fn view_without_session_id_throws_helpful_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let error = invoke_workflow(json!({ "view": true }))
        .expect_err("view without sessionId must return error")
        .to_string();
    assert!(
        error.contains("Invalid parameter combination"),
        "unexpected error: {error}"
    );
    assert!(error.contains("view=true"), "unexpected error: {error}");
    assert!(error.contains("sessionId"), "unexpected error: {error}");
}
