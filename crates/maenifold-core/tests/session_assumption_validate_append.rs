// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

#[test]
fn validate_append_input_null_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(None, Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_empty_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(Some(""), Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_whitespace_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(Some("   "), Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_null_concepts_returns_error() {
    let result = validate_assumption_append_input(Some("Valid assumption"), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("[[WikiLink]]"));
}

#[test]
fn validate_append_input_empty_concepts_array_returns_error() {
    let concepts: Vec<String> = Vec::new();
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("[[WikiLink]]"));
}

#[test]
fn validate_append_input_concept_with_full_brackets_returns_error() {
    let concepts = vec!["[[foo]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_concept_with_only_open_bracket_returns_error() {
    let concepts = vec!["[[dialogue".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_concept_with_only_close_bracket_returns_error() {
    let concepts = vec!["dialogue]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_valid_single_concept_returns_null() {
    let concepts = vec!["dialogue".to_string()];
    assert!(
        validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts)).is_none()
    );
}

#[test]
fn validate_append_input_valid_multiple_concepts_returns_null() {
    let concepts = vec![
        "dialogue".to_string(),
        "workflow-dispatch".to_string(),
        "assumption-ledger".to_string(),
    ];
    assert!(
        validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts)).is_none()
    );
}

#[test]
fn validate_append_input_bracket_error_on_second_concept_returns_error() {
    let concepts = vec!["good-concept".to_string(), "[[bad-concept]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}
