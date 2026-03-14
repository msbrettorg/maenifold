#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, frontmatter_usize, invoke_workflow, read_workflow_session, start_session,
    workflow_operations_env,
};

// T-12.0: RTM NFR-1 - keep workflow progress and metadata assertions focused and maintainable.
#[test]
fn continue_to_next_step_in_same_workflow() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Completed first step with [[workflow-execution]] and [[step-progression]].";
    let output = invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("continue workflow");
    assert!(output.contains("Step 2/3"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
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
    assert!(frontmatter.get("title").is_some(), "missing title");
    assert_eq!(
        frontmatter.get("type").and_then(YamlValue::as_str),
        Some("workflow")
    );
    assert_eq!(
        frontmatter.get("permalink").and_then(YamlValue::as_str),
        Some(session_id.as_str())
    );
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert!(frontmatter.get("queue").is_some(), "missing queue");
    assert!(
        frontmatter.get("currentWorkflow").is_some(),
        "missing currentWorkflow"
    );
    assert!(
        frontmatter.get("currentStep").is_some(),
        "missing currentStep"
    );
}

#[test]
fn continue_preserves_thoughts_in_content() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Response with [[step-output]].";
    let thoughts = "Meta thoughts with [[ambient-thinking]] and [[metacognition]].";
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "thoughts": thoughts
    }))
    .expect("continue with thoughts");
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

#[test]
fn continue_with_thoughts_but_no_conclusion_at_intermediate_step() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Response with [[intermediate-results]].";
    let thoughts = "Optional thinking with [[thought-process]].";
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "thoughts": thoughts
    }))
    .expect("continue with thoughts");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Step 2/3"), "unexpected output: {output}");
    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(content.contains(thoughts), "unexpected content: {content}");
}
