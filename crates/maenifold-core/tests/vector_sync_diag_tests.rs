use maenifold_core::json::OutputFormat;
use maenifold_core::search::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
    build_vector_failure_report, build_vector_unavailable_report, generate_concept_embeddings,
    sync_files,
};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct EnvVarReset {
    key: &'static str,
    value: Option<String>,
}

impl EnvVarReset {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        // T-QUAL-002.6b: RTM FR-25.2
        unsafe { std::env::set_var(key, value) };
        Self {
            key,
            value: previous,
        }
    }
}

impl Drop for EnvVarReset {
    fn drop(&mut self) {
        match self.value.as_deref() {
            Some(value) => {
                // T-QUAL-002.6b: RTM FR-25.2
                unsafe { std::env::set_var(self.key, value) };
            }
            None => {
                // T-QUAL-002.6b: RTM FR-25.2
                unsafe { std::env::remove_var(self.key) };
            }
        }
    }
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) -> PathBuf {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write fixture");
    path
}

fn repo_source_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
#[test]
fn fr25_2_ac2_vector_support_has_no_empty_error_swallow_blocks() {
    let source_path = repo_source_path("src/search.rs");
    let source = fs::read_to_string(&source_path).expect("read search.rs");
    let empty_swallow_pattern = Regex::new(
        r"(?s)(if let Err\([^\)]*\)\s*=\s*[^\{]+\{\s*(?:(?://[^\n]*\n)|/\*.*?\*/\s*)*\})|(Err\([^\)]*\)\s*=>\s*\{\s*(?:(?://[^\n]*\n)|/\*.*?\*/\s*)*\})",
    )
    .expect("valid regex");
    let matches = empty_swallow_pattern
        .find_iter(&source)
        .map(|m| m.as_str().to_string())
        .collect::<Vec<_>>();

    assert!(
        matches.is_empty(),
        "found empty/comment-only error-swallow block(s) in search.rs:\n{}",
        matches.join("\n---\n")
    );
}

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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

// T-QUAL-002.6b: RTM FR-25.2
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
