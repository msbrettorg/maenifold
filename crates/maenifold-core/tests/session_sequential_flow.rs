// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn session_with_revision_creates_revision_heading() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Original thought with [[initial-idea]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let revision_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Revised thought with [[refined-approach]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "isRevision": true,
            "revisesThought": 0
        }),
    );

    assert!(revision_result.contains("Added thought 1 to session:"));
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("(Revises: 0)"));
}

#[test]
fn session_with_branch_creates_branch_heading() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main branch thought with [[branching-strategy]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main branch thought 2 with [[revision-control]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "Main branch conclusion with [[thought-divergence]]."
        }),
    );

    let branch_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Branched thought with [[alternative-concept]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "sessionId": session_id,
            "branchFromThought": 0,
            "branchId": "alternative-analysis"
        }),
    );

    assert!(branch_result.contains("Created session:"));
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("(Branch: alternative-analysis)"));
}

#[test]
fn session_with_needs_more_thoughts_expands_total_thoughts() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[api-design]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let expand_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[implementation-details]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "needsMoreThoughts": true
        }),
    );

    assert!(expand_result.contains("Continue with thought 2/?"));
    assert!(expand_result.contains("extending beyond initial estimate"));
}

#[test]
fn new_session_with_invalid_parent_workflow_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking with [[workflow-validation]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": "nonexistent-workflow-12345"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("not found"));
}

#[test]
fn continue_session_with_parent_workflow_on_thought_2_returns_error() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[parent-workflow-linking]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);
    let parent_session_id = harness.create_workflow_session();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[error-handling]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "parentWorkflowId": parent_session_id
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Parent workflow can only be set on first thought"));
}
