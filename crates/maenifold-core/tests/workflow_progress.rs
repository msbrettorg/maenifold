#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, frontmatter_usize, invoke_workflow, queue_from_frontmatter,
    read_workflow_session, start_session, workflow_operations_env,
};

// T-12.0: RTM NFR-1 - keep workflow progress and metadata assertions focused and maintainable.
//
// NOTE: This test validates persisted *content* (response heading, WikiLinks) after a step
// advance — distinguishing it from `workflow_lifecycle::workflow_advances_from_step1_to_step2`
// which only checks output text and frontmatter step number.
#[test]
fn continue_to_next_step_in_same_workflow() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Completed first step with [[workflow-execution]] and [[step-progression]].";
    let output = invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("continue workflow");
    // T-12.0: RTM NFR-1 — positive step-advancement assertion replaces fragile negative error scan
    assert!(output.contains("Step 2/3"), "expected step 2/3 prompt in output: {output}");
    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
    assert!(
        content.contains("Step 1/3 Response"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[workflow-execution]]"),
        "unexpected content: {content}"
    );
}

// T-12.0: RTM NFR-1 — post-continue frontmatter must carry correct *values*, not just key presence.
// Fields that don't change after a continue (type, status) are covered by workflow_lifecycle tests.
#[test]
fn continue_creates_proper_frontmatter_metadata() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Work with [[metadata-tracking]]."
    }))
    .expect("continue workflow");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("permalink").and_then(YamlValue::as_str),
        Some(session_id.as_str()),
        "permalink must equal session id"
    );
    assert!(
        frontmatter
            .get("title")
            .and_then(YamlValue::as_str)
            .map(|t| !t.is_empty())
            .unwrap_or(false),
        "title should be non-empty after continue"
    );
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 0);
    let queue = queue_from_frontmatter(&frontmatter);
    assert_eq!(queue, vec!["test-workflow-1"]);
}

#[test]
fn continue_preserves_thoughts_in_content() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Response with [[step-output]].";
    let thoughts = "Meta thoughts with [[ambient-thinking]] and [[metacognition]].";
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "thoughts": thoughts
    }))
    .expect("continue with thoughts");
    // T-12.0: RTM NFR-1 — absorbed from test 4 (MERGE): thoughts-bearing request still advances step
    assert!(output.contains("Step 2/3"), "expected step advancement in output: {output}");
    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(content.contains(response), "unexpected content: {content}");
    assert!(content.contains(thoughts), "unexpected content: {content}");
    assert!(
        content.contains("Thoughts:"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[ambient-thinking]]"),
        "unexpected content: {content}"
    );
}
