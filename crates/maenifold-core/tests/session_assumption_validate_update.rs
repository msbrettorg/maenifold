// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

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

#[test]
fn validate_update_input_valid_uri_status_active_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("active"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_validated_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("validated"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_invalidated_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("invalidated"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_refined_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("refined"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_invalid_status_returns_error() {
    let result = validate_assumption_update_input(
        Some("memory://assumptions/2026/02/assumption-abc123"),
        Some("bogus"),
    );
    assert!(result.is_some());
    assert!(result.expect("error").contains("Invalid status"));
}

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

#[test]
fn validate_status_active_returns_null() {
    assert!(validate_assumption_status("active").is_none());
}

#[test]
fn validate_status_validated_returns_null() {
    assert!(validate_assumption_status("validated").is_none());
}

#[test]
fn validate_status_invalidated_returns_null() {
    assert!(validate_assumption_status("invalidated").is_none());
}

#[test]
fn validate_status_refined_returns_null() {
    assert!(validate_assumption_status("refined").is_none());
}

#[test]
fn validate_status_active_mixed_case_returns_null() {
    assert!(validate_assumption_status("Active").is_none());
}

#[test]
fn validate_status_validated_upper_case_returns_null() {
    assert!(validate_assumption_status("VALIDATED").is_none());
}

#[test]
fn validate_status_invalid_status_returns_error() {
    let result = validate_assumption_status("bogus");
    assert!(result.is_some());
    assert!(result.expect("error").contains("Invalid status"));
}

#[test]
fn validate_status_invalid_status_error_message_contains_all_valid_values() {
    let result = validate_assumption_status("bogus").expect("error");
    assert!(result.contains("active"));
    assert!(result.contains("validated"));
    assert!(result.contains("invalidated"));
    assert!(result.contains("refined"));
}
