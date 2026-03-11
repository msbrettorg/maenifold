//! T-9.6: RTM FR-7, FR-8 — vector sync unavailable-path diagnostics coverage.

#[path = "vector_sync_diag/support.rs"]
mod support;

// T-9.6: RTM FR-7, FR-8 — unavailable-path split from vector_sync_diag_tests.rs.
use maenifold_core::ToolRegistry;
use maenifold_core::json::OutputFormat;
use maenifold_core::search::sync_files;
use maenifold_core::tools::ToolContext;
use serde_json::json;
use support::{
    ConfigOverrideReset, EnvVarReset, TEST_MUTEX, isolated_config, write_memory_fixture,
};

#[test]
fn fr25_2_ac3_sync_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason()
{
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let _disable_vector = EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "true");

    let _file_path = write_memory_fixture(
        &cfg,
        "vector-diag/sync-ac3.md",
        "# Sync AC3\n\nReferences [[ac3-sync-concept]].\n",
    );

    let output = ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");

    assert!(
        output.contains("Sync complete:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("Vector sync diagnostics:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- STATUS: unavailable"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("- WARNING: sqlite-vec extension unavailable; semantic tooling disabled"),
        "unexpected output: {output}"
    );
}

#[test]
fn fr25_2_ac4_sync_files_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason()
 {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let _disable_vector = EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "true");

    let file_path_a = write_memory_fixture(
        &cfg,
        "vector-diag/sync-files-a.md",
        "# SyncFiles AC4 A\n\nReferences [[ac4-syncfiles-a]].\n",
    );
    let file_path_b = write_memory_fixture(
        &cfg,
        "vector-diag/sync-files-b.md",
        "# SyncFiles AC4 B\n\nReferences [[ac4-syncfiles-b]].\n",
    );

    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    let output = sync_files(&context, &[file_path_a, file_path_b]).expect("sync files");

    assert!(
        output.contains("SyncFiles complete:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("Vector sync diagnostics:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- STATUS: unavailable"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- COUNTERS_SKIPPED: concepts=2, memory=2, total=4"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"),
        "unexpected output: {output}"
    );
}

#[test]
fn fr25_2_ac3_sync_vector_unavailable_does_not_emit_failure_or_verification_sections() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let _disable_vector = EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "true");

    let _file_path = write_memory_fixture(
        &cfg,
        "vector-diag/sync-ac3-guardrail.md",
        "# Sync AC3 Guardrail\n\nReferences [[ac3-sync-guardrail]].\n",
    );

    let output = ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");

    assert!(
        output.contains("Vector sync diagnostics:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- STATUS: unavailable"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("- FAILURES:"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("Vector index verification:"),
        "unexpected output: {output}"
    );
}

#[test]
fn fr25_2_ac4_sync_files_vector_unavailable_does_not_emit_failure_or_verification_sections() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let _disable_vector = EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "true");

    let file_path = write_memory_fixture(
        &cfg,
        "vector-diag/sync-files-guardrail.md",
        "# SyncFiles AC4 Guardrail\n\nReferences [[ac4-syncfiles-guardrail]].\n",
    );

    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    let output = sync_files(&context, &[file_path]).expect("sync files");

    assert!(
        output.contains("Vector sync diagnostics:"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("- STATUS: unavailable"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("- FAILURES:"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("Vector index verification:"),
        "unexpected output: {output}"
    );
}
