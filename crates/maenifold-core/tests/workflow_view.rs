#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use workflow_support::{
    TEST_MUTEX, invoke_workflow, start_session, workflow_operations_env, workflow_tools_env,
};

// T-12.0: RTM NFR-1 — verifies that the initial queue view renders position, step counter,
// and all queued workflow names. Migrated T-12.0 traceability from the former wtt-* fixture
// twin (view_queue_shows_current_position_and_next_step) which was a strict subset of this test.
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

// T-12.0: RTM NFR-1 — after advancing one step, the view must reflect the updated step
// counter and display the next step prompt. Covers both single-workflow and multi-workflow
// queue contexts to ensure step rendering is independent of queue size.
#[test]
fn view_reflects_updated_position_after_step_advancement() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    // Case 1: single-workflow session — step advancement without queue position concerns.
    let (_, single_id) = start_session(json!("wtt-alpha"));
    invoke_workflow(json!({
        "sessionId": single_id,
        "response": "Step 1 done with [[step-tracking]]."
    }))
    .expect("advance single-workflow session");
    let output = invoke_workflow(json!({ "sessionId": single_id, "view": true }))
        .expect("view advanced single-workflow session");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");

    // Case 2: multi-workflow queue — step advancement within the first workflow of a queue.
    // Verifies the step counter still reads step 2/3 when the session has multiple queued
    // workflows, ensuring queue size does not interfere with per-workflow step display.
    let queue_json = serde_json::to_string(&["wtt-alpha", "wtt-beta"]).expect("queue json");
    let (_, queue_id) = start_session(json!(queue_json));
    invoke_workflow(json!({
        "sessionId": queue_id,
        "response": "Step work with [[view-state]]."
    }))
    .expect("advance queue session");
    let output = invoke_workflow(json!({ "sessionId": queue_id, "view": true }))
        .expect("view advanced queue session");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");
}

#[test]
fn view_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let invalid_session_id = "workflow-nonexistent-12345";
    let error = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "view": true
    }))
    .expect_err("invalid session view must return Err")
    .to_string();
    assert!(
        error.contains(invalid_session_id),
        "error should echo session id: {error}"
    );
    assert!(
        error.contains("doesn't exist"),
        "error should indicate nonexistent session: {error}"
    );
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
