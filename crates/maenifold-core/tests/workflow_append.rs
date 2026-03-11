mod workflow_support;

use serde_json::json;
use workflow_support::{
    TEST_MUTEX, invoke_workflow, queue_from_frontmatter, read_workflow_session, start_session,
    workflow_operations_env, workflow_tools_env,
};

// T-12.0: RTM NFR-1 - keep append-path coverage intact in a dedicated integration test file.
#[test]
fn append_adds_workflow_to_queue() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({ "sessionId": session_id, "append": "wtt-beta" }))
        .expect("append workflow");
    assert!(
        output.contains("Added 1 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[1], "wtt-beta");
}

#[test]
fn append_adds_multiple_workflows_as_json_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let append_json = serde_json::to_string(&["wtt-beta", "wtt-gamma"]).expect("append json");
    let output = invoke_workflow(json!({ "sessionId": session_id, "append": append_json }))
        .expect("append workflows");
    assert!(
        output.contains("Added 2 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(output.contains("wtt-gamma"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 3);
}

#[test]
fn append_valid_workflow_to_queue() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "append": "test-workflow-2"
    }))
    .expect("append valid workflow");
    assert!(
        output.contains("Added 1 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-2"),
        "unexpected output: {output}"
    );
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[1], "test-workflow-2");
}

#[test]
fn append_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let invalid_session_id = "1000000000000";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "append": "test-workflow-1"
    }))
    .expect("append invalid session output");
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
fn append_with_missing_workflow_file_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let missing = "nonexistent-workflow-xyz";
    let error = invoke_workflow(json!({
        "sessionId": session_id,
        "append": missing
    }))
    .expect_err("missing append workflow should error")
    .to_string();
    assert!(error.contains(missing), "unexpected error: {error}");
    assert!(error.contains("not found"), "unexpected error: {error}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 1);
}

#[test]
fn append_multiple_workflows_as_json_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let append_json =
        serde_json::to_string(&["test-workflow-2", "test-workflow-3"]).expect("append json");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "append": append_json
    }))
    .expect("append workflow array");
    assert!(
        output.contains("Added 2 workflow(s) to queue"),
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
    assert!(output.contains("New queue:"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(
        queue,
        vec![
            "test-workflow-1".to_string(),
            "test-workflow-2".to_string(),
            "test-workflow-3".to_string()
        ]
    );
}

#[test]
fn append_validates_all_workflows_before_queue_update() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let mixed_json = serde_json::to_string(&["test-workflow-2", "nonexistent-workflow-xyz"])
        .expect("mixed json");
    let error = invoke_workflow(json!({
        "sessionId": session_id,
        "append": mixed_json
    }))
    .expect_err("mixed append should error")
    .to_string();
    assert!(
        error.contains("nonexistent-workflow-xyz"),
        "unexpected error: {error}"
    );
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 1);
}
