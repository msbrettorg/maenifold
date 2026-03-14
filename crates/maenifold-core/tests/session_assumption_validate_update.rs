// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

// --- KEEP: URI boundary tests for validate_assumption_update_input ---

#[test]
fn validate_update_input_null_uri_returns_error() {
    let result = validate_assumption_update_input(None, None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_update_input_empty_uri_returns_error() {
    let result = validate_assumption_update_input(Some(""), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_update_input_whitespace_uri_returns_error() {
    let result = validate_assumption_update_input(Some("   "), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

// --- KEEP: Happy paths for update input with absent/empty status ---

#[test]
fn validate_update_input_valid_uri_null_status_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            None,
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_empty_status_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some(""),
        )
        .is_none()
    );
}

// --- MERGE tests 6–9: one delegation test for valid status (Rec #1) ---
// T-12.7: RTM FR-17.9 — individual status values exhaustively tested by
// validate_status_accepts_all_valid_values; one delegation probe suffices here.

#[test]
fn validate_update_input_delegates_status_validation_to_status_validator() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("active"),
        )
        .is_none(),
        "valid status should propagate None from status validator"
    );
}

// --- MERGE test 10: delegation error propagation ---

#[test]
fn validate_update_input_propagates_status_validation_error() {
    // T-12.7: RTM FR-17.9 — confirm delegation propagates error; detailed
    // error-message assertions live in validate_status_rejects_unknown_value…
    let result = validate_assumption_update_input(
        Some("memory://assumptions/2026/02/assumption-abc123"),
        Some("bogus"),
    );
    assert!(
        result.is_some(),
        "invalid status should propagate error from status validator"
    );
}

// --- KEEP: validate_assumption_read_input tests ---

#[test]
fn validate_read_input_null_uri_returns_error() {
    let result = validate_assumption_read_input(None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_read_input_empty_uri_returns_error() {
    let result = validate_assumption_read_input(Some(""));
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_read_input_valid_uri_returns_null() {
    assert!(
        validate_assumption_read_input(Some("memory://assumptions/2026/02/assumption-abc123"))
            .is_none()
    );
}

// --- MERGE tests 14–17: parameterized valid-status test (Rec #2) ---

#[test]
fn validate_status_accepts_all_valid_values() {
    // T-12.7: RTM FR-17.9 — each value is individually load-bearing;
    // a deleted match arm is caught by the per-iteration failure message.
    for status in ["active", "validated", "invalidated", "refined"] {
        assert!(
            validate_assumption_status(status).is_none(),
            "expected None for '{status}'"
        );
    }
}

// --- KEEP: case-insensitivity contract test ---

#[test]
fn validate_status_active_mixed_case_returns_null() {
    assert!(validate_assumption_status("Active").is_none());
}

// --- REMOVE: validate_status_validated_upper_case_returns_null (Rec #3) ---
// Subsumed by validate_status_active_mixed_case_returns_null — both exercise
// the same eq_ignore_ascii_case branch.

// --- MERGE + REWRITE tests 20+21 (Rec #4): actionable error with all valid values ---

#[test]
fn validate_status_rejects_unknown_value_with_actionable_error_listing_all_valid_values() {
    // T-12.7: RTM FR-17.9 / NFR-1 — error must be actionable: prefixed with
    // "ERROR:" (C# parity), include "Invalid status", and list every valid value.
    let message =
        validate_assumption_status("bogus").expect("unknown status must produce an error");
    assert!(
        message.starts_with("ERROR:"),
        "error must start with 'ERROR:' for C# parity, got: {message}"
    );
    assert!(
        message.contains("Invalid status"),
        "must say 'Invalid status', got: {message}"
    );
    for expected in ["active", "validated", "invalidated", "refined"] {
        assert!(
            message.contains(expected),
            "error should list valid value '{expected}', got: {message}"
        );
    }
}

// --- ADD: empty-string edge case (Rec #5) ---

#[test]
fn validate_status_rejects_empty_string() {
    // T-12.7: RTM FR-17.9 — empty string trimmed to "" matches no valid status.
    let result = validate_assumption_status("");
    assert!(
        result.is_some(),
        "empty string status should produce an error"
    );
}
