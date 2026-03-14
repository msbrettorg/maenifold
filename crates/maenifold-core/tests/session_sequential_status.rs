// T-12.7: RTM NFR-2
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

// Guards: completed-parent rejection — no child session created, parent unmutated
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
    // T-12.7: Verify no session was created despite the error
    assert!(!result.contains("Created session:"));
    // T-12.7: Verify parent was not mutated by the failed attempt
    let (parent_after, _) = harness.read(&parent_uri);
    assert_eq!(frontmatter_string(&parent_after, "status"), "completed");
}

// Guards: cancelled-parent rejection — both terminal states block child creation
#[test]
fn new_session_with_cancelled_parent_workflow_returns_error() {
    let harness = TestHarness::new();
    let parent_session_id = harness.create_workflow_session();
    let parent_uri = session_uri("workflow", &parent_session_id);
    let (mut parent_frontmatter, parent_content) = harness.read(&parent_uri);
    parent_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("cancelled".to_string()),
    );
    harness.write(&parent_uri, &parent_frontmatter, &parent_content);

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking within cancelled [[workflow-state]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": parent_session_id
        }),
    );

    assert!(result.starts_with("ERROR:"));
    // T-12.7: Verify no session was created
    assert!(!result.contains("Created session:"));
    // T-12.7: Verify parent was not mutated
    let (parent_after, _) = harness.read(&parent_uri);
    assert_eq!(frontmatter_string(&parent_after, "status"), "cancelled");
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
    // T-12.7: No thought should be added when cancel has no response
    assert!(!cancel_result.contains("Added thought"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, _) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");
    assert!(frontmatter.contains_key("cancelled"));
    // T-12.7: Pin thoughtCount value — only thought 0 from creation, no cancel thought
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "1");
}

// Guards: WikiLink in conclusion field is preserved verbatim in persisted content
#[test]
fn session_conclusion_preserves_wikilinks_in_content() {
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
    let (_, content) = harness.read(&uri);
    // T-12.7: Verify WikiLink appears specifically within the Conclusion section
    let conclusion_pos = content
        .find("## Conclusion")
        .expect("conclusion section must exist");
    assert!(content[conclusion_pos..].contains("[[synthesis]]"));
    // T-12.7: Verify conclusion text is also preserved
    assert!(content[conclusion_pos..].contains("This analysis demonstrates"));
}
