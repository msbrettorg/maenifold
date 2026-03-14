// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

// T-12.7: RTM NFR-1 — exact error strings for oracle parity assertions
const ERR_ASSUMPTION_REQUIRED: &str =
    "ERROR: Assumption text is required for 'append' action.";
const ERR_CONCEPTS_REQUIRED: &str =
    "ERROR: At least one [[WikiLink]] tag must be provided. Example: concepts: [\"dialogue\", \"workflow-dispatch\"]";
const ERR_BRACKETS_IN_CONCEPTS: &str =
    "ERROR: Concept tags should not include brackets. Use 'dialogue' not '[[dialogue]]'.";

// T-12.7: RTM NFR-1 — KEEP: canonical test for the None assumption branch
#[test]
fn validate_append_input_null_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(None, Some(&concepts));
    assert_eq!(
        result.as_deref(),
        Some(ERR_ASSUMPTION_REQUIRED),
        "None assumption must return the exact parity error message"
    );
}

// T-12.7: RTM NFR-1 — MERGE: empty + whitespace + tab assumption variants
#[test]
fn assumption_text_required_for_empty_variants() {
    let concepts = vec!["concept".to_string()];
    for (label, input) in [
        ("empty string", Some("")),
        ("whitespace only", Some("   ")),
        ("tab only", Some("\t")),
    ] {
        let result = validate_assumption_append_input(input, Some(&concepts));
        assert_eq!(
            result.as_deref(),
            Some(ERR_ASSUMPTION_REQUIRED),
            "assumption={label:?} must return the exact parity error message"
        );
    }
}

// T-12.7: RTM NFR-1 — KEEP: canonical test for the None concepts branch
#[test]
fn validate_append_input_null_concepts_returns_error() {
    let result = validate_assumption_append_input(Some("Valid assumption"), None);
    assert_eq!(
        result.as_deref(),
        Some(ERR_CONCEPTS_REQUIRED),
        "None concepts must return the exact parity error message"
    );
}

// T-12.7: RTM NFR-1 — KEEP: empty-vec concepts branch (separate code path from None)
#[test]
fn validate_append_input_empty_concepts_array_returns_error() {
    let concepts: Vec<String> = Vec::new();
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert_eq!(
        result.as_deref(),
        Some(ERR_CONCEPTS_REQUIRED),
        "empty concepts vec must return the exact parity error message"
    );
}

// T-12.7: RTM NFR-1 — MERGE: three bracket-pattern variants into parameterized test
#[test]
fn bracket_patterns_in_concepts_are_rejected() {
    for (label, concept) in [
        ("full brackets", "[[foo]]"),
        ("open bracket only", "[[dialogue"),
        ("close bracket only", "dialogue]]"),
    ] {
        let concepts = vec![concept.to_string()];
        let result =
            validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
        assert_eq!(
            result.as_deref(),
            Some(ERR_BRACKETS_IN_CONCEPTS),
            "concept={label:?} ({concept:?}) must return the exact parity error message"
        );
    }
}

// T-12.7: RTM NFR-1 — KEEP: happy-path anchor for well-formed input
#[test]
fn validate_append_input_valid_single_concept_returns_null() {
    let concepts = vec!["dialogue".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts));
    assert!(result.is_none(), "expected Ok but got error: {result:?}");
}

// REMOVE: validate_append_input_valid_multiple_concepts_returns_null
// Redundant — bracket_error_on_second_concept already proves multi-concept iteration.

// T-12.7: RTM NFR-1 — KEEP: proves bracket check iterates over ALL concepts
#[test]
fn validate_append_input_bracket_error_on_second_concept_returns_error() {
    let concepts = vec!["good-concept".to_string(), "[[bad-concept]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts));
    assert_eq!(
        result.as_deref(),
        Some(ERR_BRACKETS_IN_CONCEPTS),
        "bracket on second concept must return the exact parity error message"
    );
}
