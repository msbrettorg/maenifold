// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn new_session_with_closed_parent_workflow_returns_error() {
    let harness = TestHarness::new();
    let parent_session_id = harness.create_workflow_session();
    let parent_uri = session_uri("workflow", &parent_session_id);
    let (mut parent_frontmatter, parent_content) = harness.read(&parent_uri);
    parent_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    harness.write(&parent_uri, &parent_frontmatter, &parent_content);

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking within closed [[workflow-state]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": parent_session_id
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("is completed"));
}

#[test]
fn session_with_cancel_updates_status_to_cancelled() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[session-lifecycle]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 5
        }),
    );
    let session_id = extract_session_id(&create_result);

    let cancel_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Cancelling thoughts with [[workflow-cancellation]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 5,
            "sessionId": session_id,
            "cancel": true
        }),
    );

    assert!(cancel_result.contains("Thinking cancelled"));
    assert!(cancel_result.contains("Added thought 1 to session:"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, _) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");
    assert!(frontmatter.contains_key("cancelled"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
}

#[test]
fn session_with_cancel_without_response_updates_status_to_cancelled() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[cancel-workflow]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 5
        }),
    );
    let session_id = extract_session_id(&create_result);

    let cancel_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "sessionId": session_id,
            "cancel": true
        }),
    );

    assert!(cancel_result.contains("Thinking cancelled"));
    assert!(!cancel_result.starts_with("ERROR:"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, _) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");
    assert!(frontmatter.contains_key("cancelled"));
    assert!(frontmatter.contains_key("thoughtCount"));
}

#[test]
fn session_with_conclusion_completes_session() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought 1 with [[requirements-analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let complete_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought 2 with [[solution-design]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "This analysis demonstrates [[synthesis]] of concepts."
        }),
    );

    assert!(complete_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert!(frontmatter.contains_key("completed"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
    assert!(content.contains("Conclusion"));
    assert!(content.contains("[[synthesis]]"));
}
