#[path = "workflow_support/mod.rs"]
mod workflow_support;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use workflow_support::{
    TEST_MUTEX, invoke_workflow, read_workflow_session, start_session, workflow_tools_env,
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

// T-12.0: RTM NFR-1 - merged from conclusion_with_wikilinks_is_stored_in_session_content.
#[test]
fn status_transition_active_to_completed() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
    let conclusion = concat!(
        "Session concluded with [[workflow]] insights, [[maenifold]] learnings, ",
        "[[sequential-thinking]] mechanics, and [[session-management]] at scale."
    );
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Final step with [[session-management]].",
        "status": "completed",
        "conclusion": conclusion
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
    // Merged from conclusion_with_wikilinks_is_stored_in_session_content:
    // verify multiple WikiLinks in the conclusion are all persisted.
    assert!(
        content.contains("[[workflow]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[maenifold]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[sequential-thinking]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[session-management]]"),
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

// T-12.0: RTM NFR-1 - merged from continue_with_cancellation_status_terminates_early (same contract).
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

// T-12.0: RTM NFR-1 - rewrite: removed emoji assertion, renamed per eval rec 4.
#[test]
fn completion_after_intermediate_step_persists_state() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();
    let (_, session_id) = start_session(json!("wtt-alpha"));
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
        content.contains("[[session-summary]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();
    let invalid_session_id = "workflow-1234";
    let result = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "response": "Response with [[error-validation]]."
    }));
    // Invalid session ID may surface as Err (hard error) or Ok with "ERROR" in the output.
    let error_text = match result {
        Err(e) => e.to_string(),
        Ok(output) => {
            assert!(output.contains("ERROR"), "unexpected output: {output}");
            output
        }
    };
    assert!(
        error_text.contains(invalid_session_id),
        "unexpected error: {error_text}"
    );
    assert!(
        error_text.contains("doesn't exist"),
        "unexpected error: {error_text}"
    );
}
