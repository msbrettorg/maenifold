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
    // T-12.7: RTM NFR-1 — verify completed/cancelled timestamps absent on active session
    assert!(!frontmatter.contains_key("completed"));
    assert!(!frontmatter.contains_key("cancelled"));
    assert!(content.contains("Thought 0/3"));
    assert!(content.contains("test thought with [[sequential-thinking]]"));
}

// T-12.7: RTM NFR-1 — Rewritten: single-shot completion verifies file-level side effects
#[test]
fn single_thought_session_with_conclusion_completes_immediately() {
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

    // Verify file-level side effects — the session must be persisted with correct state
    let session_id = extract_session_id(&result);
    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);

    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "1");
    assert!(frontmatter.contains_key("completed"));
    assert!(content.contains("[[deep-learning]]"));
    assert!(content.contains("Conclusion"));
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

    // T-12.7: RTM NFR-1 — validation failure must not create any session files
    let session_count =
        std::fs::read_dir(&harness.config.sequential_path).map_or(0, Iterator::count);
    assert_eq!(
        session_count, 0,
        "validation failure must not create any session files"
    );
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
    // T-12.7: RTM NFR-1 — verify conclusion text persisted in session content
    assert!(content.contains("Final synthesis with [[concept-synthesis]]"));
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

    // T-12.7: RTM NFR-1 — parent stores WikiLink-formatted reference; related uses WikiLink format too
    assert!(
        frontmatter_string(&child_frontmatter, "parent").contains(&parent_session_id),
        "child 'parent' frontmatter should reference parent session ID"
    );
    assert!(
        frontmatter_sequence(&parent_frontmatter, "related")
            .iter()
            .any(|v| v.contains(&session_id)),
        "parent session 'related' should contain child session ID"
    );
}

// T-12.7: RTM NFR-1 — Moved from session_sequential_errors.rs (R-1)
#[test]
fn session_lifecycle_with_analysis_type_persists_all_frontmatter_fields() {
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

// T-12.7: RTM NFR-1 — Rewritten from session_sequential_errors.rs (R-2)
#[test]
fn new_session_renders_thoughts_field_in_session_content() {
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

    // Response WikiLinks render in the thought section
    assert!(content.contains("[[machine-learning]]"));
    assert!(content.contains("[[neural-networks]]"));
    assert!(content.contains("[[deep-learning]]"));

    // Thoughts field renders as a distinct section with its own WikiLinks
    assert!(content.contains("Thoughts:"));
    assert!(content.contains("[[AI]]"));
    assert!(content.contains("[[algorithms]]"));
    assert!(content.contains("[[data-science]]"));
}
