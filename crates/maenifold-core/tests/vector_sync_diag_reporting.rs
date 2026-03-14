//! T-9.6: RTM FR-7, FR-8 — vector sync diagnostics reporting coverage.
//!
//! NOTE: The `fr25_2_` prefix in test names refers to the *archived* FR-25.2 diagnostics
//! requirement. In the current RTM, FR-25 maps to Azure Deployment Manifests (RTM-137–140).
//! The prefix is retained for historical traceability but does not reference the active FR-25.

#[path = "vector_sync_diag/support.rs"]
mod support;

// T-9.6: RTM FR-7, FR-8 — reporting-focused split from vector_sync_diag_tests.rs.
use maenifold_core::search::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
    build_vector_failure_report, build_vector_unavailable_report, generate_concept_embeddings,
};
use rusqlite::Connection;
use support::EnvVarReset;

// T-9.6: RTM FR-7 — Test 1: unit-level validation of `build_vector_failure_report` with
// mixed concept + file failure reason codes. Renamed from `ac1` to `ac1a` to resolve naming
// collision with Test 2 (which exercises a different entry point).
#[test]
fn fr25_2_ac1a_failure_report_counters_and_reason_codes_are_rendered() {
    let mut diagnostics = VectorSyncDiagnostics::default();

    diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);

    let lines = build_vector_failure_report(&diagnostics);
    let output = lines.join("\n");

    // T-9.6: RTM FR-7 — counter accessors
    assert_eq!(diagnostics.concept_embedding_failures(), 1);
    assert_eq!(diagnostics.file_embedding_failures(), 2);
    assert_eq!(diagnostics.total_failures(), 3);

    // T-9.6: RTM FR-7 — report structure: header, status, counters, section header, reason lines
    assert!(lines.iter().any(|line| line == "Vector sync diagnostics:"));
    assert!(
        lines.iter().any(|line| line == "- STATUS: failed"),
        "failure report must emit STATUS line: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_FAILED: concepts=1, memory=2, total=3")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=0, memory=0, total=0")
    );
    assert!(
        lines.iter().any(|line| line == "- FAILURES:"),
        "failure report must emit FAILURES section header when reasons are non-empty: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON FILE_EMBEDDING_FAILED: 2")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON CONCEPT_LOOKUP_FAILED: 1")
    );
    assert!(
        !output.contains("SqliteException"),
        "diagnostics must not leak raw exception text: {output}"
    );
}

// T-9.6: RTM FR-7 — Test 2: integration test of the SQL-error → CONCEPT_LOOKUP_FAILED path
// through `generate_concept_embeddings`. Renamed from `ac1` to `ac1b` to resolve collision.
#[test]
fn fr25_2_ac1b_generate_concept_embeddings_lookup_failure_is_captured() {
    // T-9.6: RTM FR-7 — guard against MAENIFOLD_DISABLE_VECTOR leaking from parallel tests
    // or CI environment. Without this, the function takes the skip path instead of the
    // failure path and produces a confusing `0 != 1` assertion on concept_embedding_failures.
    let _env_guard = EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "false");

    let mut diagnostics = VectorSyncDiagnostics::default();
    let connection = Connection::open_in_memory().expect("in-memory sqlite");
    let single_concept = vec!["concept-a".to_string()];

    let generated =
        generate_concept_embeddings(&connection, &single_concept, true, &mut diagnostics);

    assert_eq!(generated, 0);
    assert_eq!(diagnostics.concept_embedding_failures(), 1);
    assert_eq!(diagnostics.total_failures(), 1);
    // T-9.6: RTM FR-7 — confirm skip path was NOT taken (env-var guard validation)
    assert_eq!(
        diagnostics.concept_embedding_skipped(),
        0,
        "MAENIFOLD_DISABLE_VECTOR should be false; skip path should not fire"
    );
    assert!(
        diagnostics
            .failure_reasons()
            .contains_key(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED),
        "expected failure reason missing: {:?}",
        diagnostics.failure_reasons()
    );
    // T-9.6: RTM FR-7 — assert reason count is exactly 1, not just present
    assert_eq!(
        diagnostics
            .failure_reasons()
            .get(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED),
        Some(&1),
        "reason count should be exactly 1 for a single concept lookup failure"
    );
}

// T-9.6: RTM FR-8 — Test 3: adversarial input normalization for the UNKNOWN reason path.
// Asserts that exception strings and file paths are scrubbed from diagnostic output.
#[test]
fn fr25_2_ac5a_unknown_unavailable_reason_is_normalized_and_does_not_leak_sensitive_data() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    diagnostics.record_concept_skipped(1);
    diagnostics.record_file_skipped();

    let leaked_reason = "SqliteException:/Users/brett/private/secrets.db";
    let lines = build_vector_unavailable_report(&diagnostics, leaked_reason);
    let output = lines.join("\n");

    // T-9.6: RTM FR-8 — report structure
    assert!(
        lines.iter().any(|line| line == "- STATUS: unavailable"),
        "unavailable report must emit STATUS line: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"),
        "recorded skipped counts must appear in report: {output}"
    );
    // T-9.6: RTM FR-8 — reason normalization
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON_CODE VECTOR_UNAVAILABLE_UNKNOWN")
    );
    // T-9.6: RTM FR-8 — security: sensitive data must not leak
    assert!(
        !output.contains("SqliteException"),
        "reason normalization must scrub exception text: {output}"
    );
    assert!(
        !output.contains("/Users/brett/private/secrets.db"),
        "reason normalization must scrub file paths: {output}"
    );
}

// T-9.6: RTM FR-8 — Test 3b: DISABLED_BY_CONFIG normalization — the positive companion case
// to the UNKNOWN adversarial test above. Ensures the known reason code passes through
// normalization without being mapped to UNKNOWN.
#[test]
fn fr25_2_ac5b_disabled_by_config_reason_is_normalized() {
    let diagnostics = VectorSyncDiagnostics::default();

    let lines = build_vector_unavailable_report(
        &diagnostics,
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
    );
    let output = lines.join("\n");

    assert!(
        lines.iter().any(|line| line == "- STATUS: unavailable"),
        "unavailable report must emit STATUS line: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON_CODE VECTOR_DISABLED_BY_CONFIG"),
        "known reason code DISABLED_BY_CONFIG must normalize to VECTOR_DISABLED_BY_CONFIG: {output}"
    );
}

// T-9.6: RTM FR-7 — Test 4a: failure report correctly separates skipped and failed counters
// when both are non-zero. Also asserts STATUS and reason lines.
#[test]
fn fr25_2_ac6a_failure_report_counters_are_separated() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    diagnostics.record_concept_skipped(3);
    diagnostics.record_file_skipped();

    let lines = build_vector_failure_report(&diagnostics);
    let output = lines.join("\n");

    assert!(
        lines.iter().any(|line| line == "- STATUS: failed"),
        "failure report must emit STATUS line: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_FAILED: concepts=1, memory=1, total=2")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=3, memory=1, total=4")
    );
    // T-9.6: RTM FR-7 — reason lines must appear in the failure report
    assert!(
        lines.iter().any(|line| line == "- FAILURES:"),
        "failure report must emit FAILURES section header: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON CONCEPT_EMBEDDING_FAILED: 1"),
        "concept failure reason line missing: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON FILE_EMBEDDING_FAILED: 1"),
        "file failure reason line missing: {output}"
    );
}

// T-9.6: RTM FR-8 — Test 4b: unavailable report correctly separates skipped and failed
// counters when both are non-zero. Also asserts STATUS and REASON_CODE.
#[test]
fn fr25_2_ac6b_unavailable_report_counters_are_separated() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    diagnostics.record_concept_skipped(3);
    diagnostics.record_file_skipped();

    let lines = build_vector_unavailable_report(
        &diagnostics,
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
    );
    let output = lines.join("\n");

    assert!(
        lines.iter().any(|line| line == "- STATUS: unavailable"),
        "unavailable report must emit STATUS line: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_FAILED: concepts=1, memory=1, total=2")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=3, memory=1, total=4")
    );
    // T-9.6: RTM FR-8 — reason code must appear in the unavailable report
    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON_CODE VECTOR_DISABLED_BY_CONFIG"),
        "unavailable report must include REASON_CODE for known reason: {output}"
    );
}

#[test]
fn fr7_t20_1_concept_embedding_failure_details_are_sampled_and_rendered() {
    let mut diagnostics = VectorSyncDiagnostics::default();

    // T-20.1: RTM FR-7 — sync diagnostics should expose capped actionable detail for
    // [[CONCEPT_EMBEDDING_FAILED]] without disturbing existing counters or reason codes.
    diagnostics.record_concept_failure_with_detail(
        VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
        "tokenizer encode failed:\nmissing [UNK] token in tokenizer.json",
    );
    diagnostics.record_concept_failure_with_detail(
        VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
        "onnx runtime error: invalid input_ids tensor shape",
    );
    diagnostics.record_concept_failure_with_detail(
        VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
        "embedding model not found: expected a non-empty *.onnx file under assets/models",
    );
    diagnostics.record_concept_failure_with_detail(
        VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
        "onnx runtime error: this extra sampled detail should be dropped by the cap",
    );
    diagnostics.record_concept_failure_with_detail(
        VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
        "onnx runtime error: invalid input_ids tensor shape",
    );

    let lines = build_vector_failure_report(&diagnostics);
    let output = lines.join("\n");

    assert_eq!(diagnostics.concept_embedding_failures(), 5);

    // T-20.1: RTM FR-7 — explicit deduplication check: 5 calls recorded but only 3 distinct
    // detail strings should be stored (duplicate is deduplicated, 4th unique is over cap).
    assert_eq!(
        diagnostics
            .concept_failure_details()
            .get(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED)
            .map(|v| v.len()),
        Some(3),
        "duplicate detail strings should be deduplicated, cap should hold at 3"
    );

    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON CONCEPT_EMBEDDING_FAILED: 5")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "- FAILURE_DETAILS CONCEPT_EMBEDDING_FAILED:")
    );

    // T-20.1: RTM FR-7 — verify "- DETAIL " prefix format (callers parsing the report depend on it)
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("- DETAIL tokenizer encode")),
        "detail lines must use '- DETAIL ' prefix format: {output}"
    );

    assert!(output.contains("tokenizer encode failed: missing [UNK] token in tokenizer.json"));
    assert!(output.contains("onnx runtime error: invalid input_ids tensor shape"));
    assert!(output.contains(
        "embedding model not found: expected a non-empty *.onnx file under assets/models"
    ));
    assert!(
        !output.contains("extra sampled detail should be dropped by the cap"),
        "detail sampling should cap noisy reports: {output}"
    );
}
