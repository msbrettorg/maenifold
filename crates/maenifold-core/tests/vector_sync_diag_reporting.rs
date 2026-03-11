//! T-9.6: RTM FR-7, FR-8 — vector sync diagnostics reporting coverage.

// T-9.6: RTM FR-7, FR-8 — reporting-focused split from vector_sync_diag_tests.rs.
use maenifold_core::search::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
    build_vector_failure_report, build_vector_unavailable_report, generate_concept_embeddings,
};
use rusqlite::Connection;

#[test]
fn fr25_2_ac1_vector_sync_failures_are_counted_and_surfaced_in_diagnostics_output() {
    let mut diagnostics = VectorSyncDiagnostics::default();

    diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);

    let lines = build_vector_failure_report(&diagnostics);
    let output = lines.join("\n");

    assert_eq!(diagnostics.concept_embedding_failures(), 1);
    assert_eq!(diagnostics.file_embedding_failures(), 2);
    assert_eq!(diagnostics.total_failures(), 3);

    assert!(lines.iter().any(|line| line == "Vector sync diagnostics:"));
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

#[test]
fn fr25_2_ac1_generate_concept_embeddings_failure_is_captured_with_reason() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    let connection = Connection::open_in_memory().expect("in-memory sqlite");
    let single_concept = vec!["concept-a".to_string()];

    let generated =
        generate_concept_embeddings(&connection, &single_concept, true, &mut diagnostics);

    assert_eq!(generated, 0);
    assert_eq!(diagnostics.concept_embedding_failures(), 1);
    assert_eq!(diagnostics.total_failures(), 1);
    assert!(
        diagnostics
            .failure_reasons()
            .contains_key(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED),
        "expected failure reason missing: {:?}",
        diagnostics.failure_reasons()
    );
}

#[test]
fn fr25_2_ac5_vector_unavailable_reason_codes_are_normalized_and_do_not_leak_exception_or_paths() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    diagnostics.record_concept_skipped(1);
    diagnostics.record_file_skipped();

    let leaked_reason = "SqliteException:/Users/brett/private/secrets.db";
    let lines = build_vector_unavailable_report(&diagnostics, leaked_reason);
    let output = lines.join("\n");

    assert!(
        lines
            .iter()
            .any(|line| line == "- REASON_CODE VECTOR_UNAVAILABLE_UNKNOWN")
    );
    assert!(
        !output.contains("SqliteException"),
        "reason normalization must scrub exception text: {output}"
    );
    assert!(
        !output.contains("/Users/brett/private/secrets.db"),
        "reason normalization must scrub file paths: {output}"
    );
}

#[test]
fn fr25_2_ac6_vector_diagnostics_clearly_separate_skipped_and_failed_counters() {
    let mut diagnostics = VectorSyncDiagnostics::default();
    diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED);
    diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    diagnostics.record_concept_skipped(3);
    diagnostics.record_file_skipped();

    let unavailable_lines = build_vector_unavailable_report(
        &diagnostics,
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
    );
    let failure_lines = build_vector_failure_report(&diagnostics);

    assert!(
        unavailable_lines
            .iter()
            .any(|line| line == "- COUNTERS_FAILED: concepts=1, memory=1, total=2")
    );
    assert!(
        unavailable_lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=3, memory=1, total=4")
    );
    assert!(
        failure_lines
            .iter()
            .any(|line| line == "- COUNTERS_FAILED: concepts=1, memory=1, total=2")
    );
    assert!(
        failure_lines
            .iter()
            .any(|line| line == "- COUNTERS_SKIPPED: concepts=3, memory=1, total=4")
    );
}
