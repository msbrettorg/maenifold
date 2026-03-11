mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, invoke_workflow, read_workflow_session, start_session, workflow_operations_env,
    workflow_tools_env,
};

// T-12.0: RTM NFR-1 - preserve completion, cancellation, and validation coverage after the split.
#[test]
fn workflow_response_without_wikilinks_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Completed step 1 without any double-bracket concepts."
    }))
    .expect("workflow validation output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLink]]"),
        "unexpected output: {output}"
    );
}

#[test]
fn status_transition_active_to_completed() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Final step with [[session-management]].",
        "status": "completed",
        "conclusion": "Session concluded with [[workflow]] insights and [[maenifold]] learnings."
    }))
    .expect("complete workflow");
    assert!(output.contains("completed"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        frontmatter.get("completed").is_some(),
        "missing completed timestamp"
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[maenifold]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn completion_without_conclusion_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Done with [[workflow]].",
        "status": "completed"
    }))
    .expect("completion validation output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("Conclusion required"),
        "unexpected output: {output}"
    );
}

#[test]
fn completion_with_conclusion_lacking_wikilinks_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Done with [[workflow]].",
        "status": "completed",
        "conclusion": "This is a conclusion without any WikiLinks."
    }))
    .expect("completion validation output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLinks]]"),
        "unexpected output: {output}"
    );
}

#[test]
fn status_transition_active_to_cancelled() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Cancelling this session with [[cancellation-reason]].",
        "status": "cancelled"
    }))
    .expect("cancel workflow");
    assert!(output.contains("cancelled"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("cancelled")
    );
    assert!(
        frontmatter.get("cancelled").is_some(),
        "missing cancelled timestamp"
    );
}

#[test]
fn conclusion_with_wikilinks_is_stored_in_session_content() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let conclusion = concat!(
        "Completed with insights about [[sequential-thinking]] and [[workflow]] mechanics. ",
        "Key findings: [[maenifold]] enables [[session-management]] at scale."
    );
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[workflow]].",
        "status": "completed",
        "conclusion": conclusion
    }))
    .expect("complete workflow");
    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[sequential-thinking]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[workflow]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[maenifold]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[session-management]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_with_completion_status_finalizes_session() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Step 1 work with [[task-completion]].";
    let conclusion = "Session complete with [[session-summary]] and [[lessons-learned]].";
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("record first response");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "status": "completed",
        "conclusion": conclusion
    }))
    .expect("complete workflow session");
    assert!(output.contains('✅'), "unexpected output: {output}");
    assert!(output.contains("completed"), "unexpected output: {output}");
    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        frontmatter.get("completed").is_some(),
        "missing completed timestamp"
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[session-summary]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_with_cancellation_status_terminates_early() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Cancelled work with [[cancellation-reason]].",
        "status": "cancelled"
    }))
    .expect("cancel workflow session");
    assert!(output.contains('❌'), "unexpected output: {output}");
    assert!(output.contains("cancelled"), "unexpected output: {output}");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("cancelled")
    );
    assert!(
        frontmatter.get("cancelled").is_some(),
        "missing cancelled timestamp"
    );
}

#[test]
fn continue_without_concepts_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "This response has no double-bracket concepts."
    }))
    .expect("concept validation output");
    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLink]]"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("knowledge graph"),
        "unexpected output: {output}"
    );
}

#[test]
fn continue_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();
    let invalid_session_id = "workflow-1234";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "response": "Response with [[error-validation]]."
    }))
    .expect("invalid session output");
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
