#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, frontmatter_usize, invoke_workflow, read_workflow_session, start_session,
    workflow_tools_env,
};

// T-12.0: RTM NFR-1 - keep workflow lifecycle coverage intact while splitting the monolithic test file.
#[test]
fn learn_mode_short_circuits_execution_without_creating_session() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let output = invoke_workflow(json!({ "learn": true })).expect("learn mode");
    assert!(!output.is_empty(), "expected learn output");
    assert!(
        !output.contains("Created session:"),
        "learn mode must not create a session: {output}"
    );
    assert!(
        !output.contains("Invalid parameter combination"),
        "learn mode must bypass operation guidance: {output}"
    );
    assert!(
        !output.contains("Cannot provide both"),
        "learn mode must bypass parameter validation: {output}"
    );
}

#[test]
fn workflow_with_workflow_id_creates_new_session() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let output = invoke_workflow(json!({ "workflowId": "wtt-alpha" })).expect("start workflow");
    assert!(
        output.contains("Created session:"),
        "unexpected output: {output}"
    );
    assert!(output.contains("Step 1/"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
}

#[test]
fn workflow_created_session_is_readable_via_markdown_io() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-beta"));
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert_eq!(
        frontmatter.get("type").and_then(YamlValue::as_str),
        Some("workflow")
    );
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 0);
}

#[test]
fn workflow_with_nonexistent_workflow_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let error = invoke_workflow(json!({ "workflowId": "does-not-exist-xyz" }))
        .expect_err("missing workflow should error")
        .to_string();
    assert!(
        error.contains("does-not-exist-xyz"),
        "unexpected error: {error}"
    );
    assert!(error.contains("not found"), "unexpected error: {error}");
}

#[test]
fn workflow_with_both_workflow_id_and_session_id_throws() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let error = invoke_workflow(json!({
        "workflowId": "wtt-alpha",
        "sessionId": "workflow-123456789"
    }))
    .expect_err("mixed start+continue should error")
    .to_string();
    assert!(
        error.contains("Cannot provide both workflowId and sessionId"),
        "unexpected error: {error}"
    );
    assert!(error.contains("sessionId"), "unexpected error: {error}");
    assert!(error.contains("workflowId"), "unexpected error: {error}");
}

#[test]
fn workflow_advances_from_step1_to_step2() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Completed step 1 with [[workflow]] and [[session-management]]."
    }))
    .expect("continue workflow");
    assert!(output.contains("Step 2/3"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
}

// T-15.14: RTM FR-8 — no-parameter call must return error, not success with help text.
#[test]
fn workflow_with_no_parameters_returns_invalid_combination_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let error = invoke_workflow(json!({}))
        .expect_err("no-parameter call must return error")
        .to_string();
    assert!(
        error.contains("Invalid parameter combination"),
        "unexpected error: {error}"
    );
    assert!(error.contains("workflowId"), "unexpected error: {error}");
    assert!(error.contains("sessionId"), "unexpected error: {error}");
}

// T-15.14: RTM FR-8 — session-id-only call must return error, not success with help text.
#[test]
fn workflow_with_only_session_id_returns_invalid_combination_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let error = invoke_workflow(json!({ "sessionId": session_id }))
        .expect_err("session-id-only call must return error")
        .to_string();
    assert!(
        error.contains("Invalid parameter combination"),
        "unexpected error: {error}"
    );
    assert!(error.contains("view=true"), "unexpected error: {error}");
}
