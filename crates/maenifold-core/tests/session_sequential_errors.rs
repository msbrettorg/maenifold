// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn complete_session_lifecycle_create_continue_complete_succeeds() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Analyzing problem space with [[architecture]] and [[design-patterns]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3,
            "analysisType": "architecture"
        }),
    );
    let session_id = extract_session_id(&create_result);

    let continue_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Implementing solution with [[implementation]] details and [[testing]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "sessionId": session_id,
            "thoughts": "Considering [[performance]] implications."
        }),
    );
    assert!(continue_result.contains("Added thought 1 to session:"));

    let complete_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Reviewing and refining with [[quality-assurance]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 2,
            "totalThoughts": 3,
            "sessionId": session_id,
            "conclusion": "Concluded that [[modular-design]] with [[robust-testing]] is the best approach."
        }),
    );
    assert!(complete_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert_eq!(frontmatter_string(&frontmatter, "type"), "sequential");
    assert_eq!(
        frontmatter_string(&frontmatter, "analysisType"),
        "architecture"
    );
    assert!(frontmatter.contains_key("completed"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "3");
    assert!(content.contains("Thought 0/3"));
    assert!(content.contains("Thought 1/3"));
    assert!(content.contains("Thought 2/3"));
    assert!(content.contains("Analyzing problem space"));
    assert!(content.contains("Implementing solution"));
    assert!(content.contains("Reviewing and refining"));
    assert!(content.contains("Conclusion"));
    assert!(content.contains("[[modular-design]]"));
}

#[test]
fn new_session_extracts_concepts_correctly() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Exploring [[machine-learning]], [[neural-networks]], and [[deep-learning]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "thoughts": "Related to [[AI]], [[algorithms]], and [[data-science]]."
        }),
    );

    let session_id = extract_session_id(&result);
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("machine-learning"));
    assert!(content.contains("Thoughts:"));
    assert!(content.contains("AI"));
}

#[test]
fn continue_session_with_nonexistent_session_id_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[validation-testing]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "sessionId": "session-9999999999999-12345"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("missing"));
}

#[test]
fn continue_session_with_invalid_session_id_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[session-validation]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            // T-14.10: RTM FR-16 - suffix-free legacy IDs are valid, so keep this case truly malformed.
            "sessionId": "session-not-a-timestamp"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Invalid sessionId format"));
}

#[test]
fn session_with_branch_from_thought_without_branch_id_returns_error() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main thought with [[multi-agent-coordination]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Branched thought with [[alternative-approach]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "sessionId": session_id,
            "branchFromThought": 0,
            "branchId": null
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("branchId required"));
    assert!(result.contains("multi-agent coordination"));
}
