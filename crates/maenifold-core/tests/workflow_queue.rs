#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, frontmatter_usize, invoke_workflow, queue_from_frontmatter, read_workflow_session,
    start_session, workflow_operations_env, workflow_tools_env,
};

// T-12.0: RTM NFR-1 - split queue progression coverage into a focused integration test module.
#[test]
fn serial_queue_starts_with_first_workflow_in_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let queue_json = serde_json::to_string(&["wtt-alpha", "wtt-beta"]).expect("queue json");
    let (output, session_id) = start_session(json!(queue_json));
    assert!(output.contains("Step 1/3"), "unexpected output: {output}");
    assert!(
        output.contains("Created session:"),
        "unexpected output: {output}"
    );
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    let queue = queue_from_frontmatter(&frontmatter);
    assert_eq!(queue, vec!["wtt-alpha".to_string(), "wtt-beta".to_string()]);
}

// T-12.0: RTM NFR-1 - serial_queue_advances_*, serial_queue_completes_*, and
// serial_queue_exhausted_* removed: each was a pairwise duplicate of the stronger
// continue_* counterpart exercising deeper fixture paths. See workflow_queue eval.

#[test]
fn continue_to_next_workflow_when_current_completes() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let queue_json =
        serde_json::to_string(&["test-workflow-3", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let result1 = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[workflow-transition]]."
    }))
    .expect("continue first step");
    assert!(result1.contains("Step 2/2"), "unexpected output: {result1}");
    let result2 = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 2 done with [[queue-advancement]]."
    }))
    .expect("transition to next workflow");
    assert!(result2.contains("Step 1/2"), "unexpected output: {result2}");
    assert!(!result2.contains("ERROR"), "unexpected output: {result2}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 1);
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
}

#[test]
fn queue_exhausted_without_conclusion_prompts_for_conclusion() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-3"));
    let response = "Work with [[completion-state]].";
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("advance to final step");
    let output = invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("queue exhaustion output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("All workflows complete"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("conclusion parameter"),
        "unexpected output: {output}"
    );
}

#[test]
fn continue_updates_current_step_index_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        0
    );
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("continue step 1");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        1
    );
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("continue step 2");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        2
    );
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("transition to next workflow");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 1);
}

#[test]
fn continue_completes_all_workflows_with_conclusion() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let queue_json =
        serde_json::to_string(&["test-workflow-3", "test-workflow-3"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let response = "Step work with [[workflow-completion]].";
    let conclusion = "Final synthesis with [[final-summary]] and [[key-takeaways]].";
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("first workflow step 1");
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("first workflow step 2");
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("second workflow step 1");
    let penultimate = invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("queue exhaustion warning");
    assert!(
        penultimate.contains("All workflows complete"),
        "unexpected output: {penultimate}"
    );
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "conclusion": conclusion
    }))
    .expect("complete all workflows");
    assert!(output.contains('✅'), "unexpected output: {output}");
    assert!(
        output.contains("All workflows"),
        "unexpected output: {output}"
    );
    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[final-summary]]"),
        "unexpected content: {content}"
    );
}
