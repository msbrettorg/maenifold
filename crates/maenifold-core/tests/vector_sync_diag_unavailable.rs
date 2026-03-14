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

// T-9.6: Shared assertion helper — reduces drift if diagnostic format changes.
fn assert_vector_unavailable_diagnostics(
    output: &str,
    prefix: &str,
    concepts: usize,
    memory: usize,
) {
    let total = concepts + memory;
    assert!(
        output.contains(&format!("{prefix} complete:")),
        "expected '{prefix} complete:' in output: {output}"
    );
    assert!(
        output.contains("Vector sync diagnostics:"),
        "expected 'Vector sync diagnostics:' in output: {output}"
    );
    assert!(
        output.contains("- STATUS: unavailable"),
        "expected '- STATUS: unavailable' in output: {output}"
    );
    assert!(
        output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"),
        "expected zero failure counters in output: {output}"
    );
    assert!(
        output.contains(&format!(
            "- COUNTERS_SKIPPED: concepts={concepts}, memory={memory}, total={total}"
        )),
        "expected skipped counters concepts={concepts}, memory={memory}, total={total} in output: {output}"
    );
    assert!(
        output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"),
        "expected VECTOR_DISABLED_BY_CONFIG reason code in output: {output}"
    );
    // Negative: failure section must be absent on the unavailable path
    assert!(
        !output.contains("- FAILURES:"),
        "unexpected FAILURES section in output: {output}"
    );
}

// T-9.6: RTM FR-7 — Sync tool emits structured unavailable diagnostics (merged from guardrail test).
#[test]
fn sync_tool_vector_disabled_emits_structured_unavailable_diagnostics() {
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

    assert_vector_unavailable_diagnostics(&output, "Sync", 1, 1);

    // Negative: verification section must be absent on the unavailable path
    assert!(
        !output.contains("Vector index verification:"),
        "unexpected verification section in output: {output}"
    );
}

// T-9.6: RTM FR-8 — sync_files emits structured unavailable diagnostics (merged from guardrail test).
#[test]
fn sync_files_vector_disabled_emits_structured_unavailable_diagnostics() {
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

    assert_vector_unavailable_diagnostics(&output, "SyncFiles", 2, 2);
}
