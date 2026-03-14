// T-12.7: RTM NFR-1
//
// File contract: SequentialThinking error-path tests.
// All tests in this file assert that SequentialThinking returns `ERROR:` for
// invalid session identity inputs (nonexistent, malformed, or missing required fields).
// Happy-path and lifecycle tests belong in `session_sequential_lifecycle.rs`.
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

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
    assert!(result.contains("missing. Start with thoughtNumber=0"));
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
