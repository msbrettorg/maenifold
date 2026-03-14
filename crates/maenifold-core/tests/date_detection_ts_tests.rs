// T-DATE-001: RTM FR-005, FR-006 — Ported timestamp regression tests from the C# oracle.
// NOTE: FR-005 and FR-006 are legacy C# oracle IDs not present in the current RTM.md.
// These map to behavioral contracts in thinking/formatting.rs and thinking/session.rs.

#[path = "session/support.rs"]
mod support;

use regex::Regex;
use serde_json::json;
use support::*;

#[test]
fn sequential_thought_timestamp_includes_utc_suffix() {
    let harness = TestHarness::new();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Test thought with [[utc-timestamp-verification]]",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );

    let session_id = extract_session_id(&result);
    let (_, content) = harness.read(&session_uri("sequential", &session_id));

    // T-DATE-001: RTM FR-005 — Human-readable sequential thought timestamps must include UTC.
    let utc_timestamp_pattern =
        Regex::new(r"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC\*").expect("regex");
    assert!(
        utc_timestamp_pattern.is_match(&content),
        "content should contain timestamp with UTC suffix. Content:\n{content}"
    );

    // T-DATE-001: RTM FR-005 — No italicized timestamp may omit the UTC suffix (merged from Test 2).
    let bare_timestamp_pattern =
        Regex::new(r"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\*").expect("regex");
    assert!(
        !bare_timestamp_pattern.is_match(&content),
        "content should not contain timestamp without UTC suffix. Content:\n{content}"
    );
}

#[test]
fn completed_session_has_iso8601_completed_field_in_frontmatter() {
    let harness = TestHarness::new();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[iso8601-completion-test]]",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Final synthesis with [[conclusion-verification]]"
        }),
    );

    assert!(
        result.contains("Thinking complete"),
        "session should complete"
    );

    let session_id = extract_session_id(&result);
    let (frontmatter, _) = harness.read(&session_uri("sequential", &session_id));

    // T-DATE-001: RTM FR-006 — Completed sessions must persist ISO 8601 completion timestamps.
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");

    let completed_value = frontmatter_string(&frontmatter, "completed");
    assert!(
        !completed_value.is_empty(),
        "completed value should not be null or empty"
    );

    // R-1: Regex now requires timezone designator (Z or ±HH:MM) to guard UTC persistence.
    let iso8601_utc_pattern =
        Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$")
            .expect("regex");
    assert!(
        iso8601_utc_pattern.is_match(&completed_value),
        "completed timestamp must be ISO 8601 with timezone designator. Got: {completed_value}"
    );
}

#[test]
fn cancelled_session_has_iso8601_cancelled_field_in_frontmatter() {
    let harness = TestHarness::new();

    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[cancel-iso8601-test]]",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3
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

    assert!(
        cancel_result.contains("Thinking cancelled"),
        "session should be cancelled"
    );

    let (frontmatter, _) = harness.read(&session_uri("sequential", &session_id));

    // T-DATE-001: RTM FR-006 — Cancelled sessions must persist ISO 8601 cancellation timestamps.
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");

    let cancelled_value = frontmatter_string(&frontmatter, "cancelled");
    assert!(
        !cancelled_value.is_empty(),
        "cancelled value should not be null or empty"
    );

    // R-1: Regex now requires timezone designator (Z or ±HH:MM) to guard UTC persistence.
    let iso8601_utc_pattern =
        Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$")
            .expect("regex");
    assert!(
        iso8601_utc_pattern.is_match(&cancelled_value),
        "cancelled timestamp must be ISO 8601 with timezone designator. Got: {cancelled_value}"
    );
}
