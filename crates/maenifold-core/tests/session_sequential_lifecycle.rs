// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn new_session_with_concepts_creates_session_file() {
    let harness = TestHarness::new();
    let response =
        "This is a test thought with [[sequential-thinking]] and [[session-management]] tags.";

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": response,
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3
        }),
    );

    assert!(result.contains("Created session:"));
    assert!(result.contains("Continue with thought 1/3"));

    let session_id = extract_session_id(&result);
    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);

    assert!(frontmatter_string(&frontmatter, "title").contains("Sequential Thinking Session"));
    assert_eq!(frontmatter_string(&frontmatter, "type"), "sequential");
    assert_eq!(frontmatter_string(&frontmatter, "status"), "active");
    assert_eq!(frontmatter_string(&frontmatter, "permalink"), session_id);
    assert!(content.contains("Thought 0/3"));
    assert!(content.contains("test thought with [[sequential-thinking]]"));
}

#[test]
fn new_session_with_concepts_in_response_passes_validation() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Analyzing [[machine-learning]] algorithms with [[neural-networks]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Concluded that [[deep-learning]] is important."
        }),
    );

    assert!(result.contains("Created session:"));
    assert!(result.contains("Thinking complete"));
    assert!(!result.starts_with("ERROR:"));
}

#[test]
fn new_session_without_concepts_fails_validation() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "This thought has no concepts at all.",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Must include [[WikiLinks]]"));
}

#[test]
fn continue_session_with_valid_session_id_appends_thought() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[data-analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let continue_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[pattern-recognition]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "Final synthesis with [[concept-synthesis]]."
        }),
    );

    assert!(continue_result.contains("Added thought 1 to session:"));
    assert!(continue_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert!(content.contains("Thought 0/2"));
    assert!(content.contains("Thought 1/2"));
    assert!(content.contains("First thought with [[data-analysis]]"));
    assert!(content.contains("Second thought with [[pattern-recognition]]"));
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
}

#[test]
fn new_session_with_parent_workflow_id_creates_bidirectional_links() {
    let harness = TestHarness::new();
    let parent_session_id = harness.create_workflow_session();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking within workflow with [[analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": parent_session_id
        }),
    );

    let session_id = extract_session_id(&result);
    let child_uri = session_uri("sequential", &session_id);
    let parent_uri = session_uri("workflow", &parent_session_id);

    let (child_frontmatter, _) = harness.read(&child_uri);
    let (parent_frontmatter, _) = harness.read(&parent_uri);

    assert!(frontmatter_string(&child_frontmatter, "parent").contains(&parent_session_id));
    assert!(
        frontmatter_sequence(&parent_frontmatter, "related")
            .iter()
            .any(|value| value.contains(&session_id))
    );
}
