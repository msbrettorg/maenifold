//! T-332.9 regression coverage for artifact-first conditional predicates.
//! The deprecated helper remains attribute-marked; the smoke test below uses
//! `#[allow(deprecated)]` because the compiler warning is the evidence.

#[allow(deprecated)]
use super::conditional_output::extract_conditional_json_value;
use super::mapping_resolve::{ConditionalPredicateResult, conditional_predicate_result};

#[test]
fn conditional_edge_with_source_output_artifact_evaluates_directly() {
    let artifact = serde_json::json!({"decision":"approve"});
    let output = Some("not json at all");
    let result = conditional_predicate_result(
        Some(&artifact),
        output,
        "decision",
        &serde_json::json!("approve"),
    );

    assert_eq!(result, ConditionalPredicateResult::Satisfied);
}

#[test]
fn conditional_edge_fallback_parses_output_string_as_json() {
    let output = Some(r#"{"decision":"approve"}"#);
    let result =
        conditional_predicate_result(None, output, "decision", &serde_json::json!("approve"));

    assert_eq!(result, ConditionalPredicateResult::Satisfied);
}

#[test]
fn conditional_edge_with_no_artifact_and_unparseable_output_does_not_fire() {
    let result = conditional_predicate_result(
        None,
        Some("not json at all"),
        "decision",
        &serde_json::json!("approve"),
    );

    assert_eq!(result, ConditionalPredicateResult::Unparseable);
}

#[test]
#[allow(deprecated)]
fn extract_conditional_json_value_marked_deprecated() {
    assert_eq!(
        extract_conditional_json_value(r#"{"decision":"approve"}"#),
        Some(serde_json::json!({"decision":"approve"}))
    );
}
