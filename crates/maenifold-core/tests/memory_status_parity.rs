// T-13.4: RTM FR-7 — MemoryStatus output parity tests.
// Verify the Rust MemoryStatus tool returns all fields matching C# SystemTools.MemoryStatus:
// folder count, total size, concept/relation/mention counts, DB size.

use maenifold_core::error::MaenifoldError;
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    #[allow(dead_code)]
    config: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&config).expect("assets");
    GraphDatabase::initialize_database(&config).expect("db init");
    TestEnv { _tmp: tmp, config }
}

fn invoke_markdown(name: &str, payload: serde_json::Value) -> Result<String, MaenifoldError> {
    ToolRegistry::global().try_invoke(name, &payload, OutputFormat::Markdown)
}

/// T-13.4: RTM FR-7 — Parse the integer value following a label like "- Total folders: 1".
/// Returns None if the label is not found or the value cannot be parsed.
fn parse_label_int(output: &str, label: &str) -> Option<i64> {
    output.lines().find_map(|line| {
        let after = line.strip_prefix(label)?;
        after.trim().parse::<i64>().ok()
    })
}

/// T-13.4: RTM FR-7 — Parse the float value between a label prefix and a suffix.
/// E.g. parse_label_float(output, "- Total size: ", " MB") extracts 0.01 from "- Total size: 0.01 MB".
fn parse_label_float(output: &str, prefix: &str, suffix: &str) -> Option<f64> {
    output.lines().find_map(|line| {
        let after = line.strip_prefix(prefix)?;
        let before_suffix = after.strip_suffix(suffix)?;
        before_suffix.trim().parse::<f64>().ok()
    })
}

// T-13.4: RTM FR-7 — MemoryStatus folder count equals exactly 1 after creating one subdirectory.
#[test]
fn memory_status_contains_folder_count() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Create exactly one subdirectory with a file to verify folder counting.
    let sub = env.config.memory_path.join("test-folder");
    fs::create_dir_all(&sub).expect("create subdir");
    fs::write(sub.join("note.md"), "# Test note\n\nContent").expect("write file");

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // T-13.4: RTM FR-7 — Assert the exact folder count, not just label presence.
    // The isolated env memory_path starts empty; we created exactly 1 subdirectory.
    let folder_count = parse_label_int(&result, "- Total folders: ")
        .unwrap_or_else(|| panic!("could not parse folder count from output: {result}"));
    assert_eq!(
        folder_count, 1,
        "expected exactly 1 folder after creating one subdirectory: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus total size is > 0.0 MB after writing a file.
#[test]
fn memory_status_contains_total_size() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Write a file large enough to exceed 0.00 MB when formatted with {:.2}.
    // Need > 0.005 MB = 5,243 bytes to round up to 0.01 MB.
    let content = "# Size test\n\n".to_string() + &"x".repeat(6000);
    fs::write(env.config.memory_path.join("size-test.md"), &content).expect("write file");

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // T-13.4: RTM FR-7 — Parse the float value and assert it is > 0.0, not just label presence.
    let total_size = parse_label_float(&result, "- Total size: ", " MB")
        .unwrap_or_else(|| panic!("could not parse total size from output: {result}"));
    assert!(
        total_size > 0.0,
        "total size should be > 0.0 MB after writing a file, got {total_size}: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus graph section reports valid counts and DB size.
// Merged from: contains_graph_stats + contains_db_size (both test ### Graph Database section).
// After isolated_env() initializes the DB schema with no data, counts should be exactly 0
// and DB size should be > 0.0 (the file exists with schema tables).
#[test]
fn memory_status_graph_section_reports_valid_counts_and_sizes() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // T-13.4: RTM FR-7 — Section header present (downstream MCP consumers parse this).
    assert!(
        result.contains("### Graph Database"),
        "output should contain Graph Database section: {result}"
    );

    // T-13.4: RTM FR-7 — Concept/relation/mention counts are exactly 0 in a fresh DB.
    assert!(
        result.contains("- Concepts: 0"),
        "concepts should be 0 in fresh DB: {result}"
    );
    assert!(
        result.contains("- Relations: 0"),
        "relations should be 0 in fresh DB: {result}"
    );
    assert!(
        result.contains("- Mentions: 0"),
        "mentions should be 0 in fresh DB: {result}"
    );

    // T-13.4: RTM FR-7 — DB file size is parseable and > 0.0 (schema tables occupy space).
    let db_size = parse_label_float(&result, "- Database size: ", " MB")
        .unwrap_or_else(|| panic!("could not parse DB size from output: {result}"));
    assert!(
        db_size > 0.0,
        "database size should be > 0.0 MB after schema init, got {db_size}: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus with no DB shows "Not initialized" message.
#[test]
fn memory_status_without_db_shows_not_initialized() {
    let _guard = TEST_MUTEX.lock();

    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    // Point to a non-existent database path.
    Config::set_database_path(tmp.path().join("nonexistent.db"));
    let config = Config::current();
    config.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&config).expect("assets");

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("Not initialized (run sync first)"),
        "should show not-initialized message when DB missing: {result}"
    );

    Config::reset_overrides();
}

// T-13.4: RTM FR-7 — MemoryStatus markdown header is present.
#[test]
fn memory_status_has_correct_header() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.starts_with("# Memory System Status"),
        "output should start with the correct header: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus file count reflects actual files written.
// The isolated env memory_path starts empty (assets go to assets_path, not memory_path).
// After writing 3 .md files, the count must be >= 3.
#[test]
fn memory_status_file_count_reflects_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Write 3 .md files into memory root.
    for i in 0..3 {
        fs::write(
            env.config.memory_path.join(format!("file{i}.md")),
            format!("# File {i}\n\nContent"),
        )
        .expect("write file");
    }

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // T-13.4: RTM FR-7 — Parse the file count and assert >= 3 to verify our files are counted.
    let file_count = parse_label_int(&result, "- Total files: ")
        .unwrap_or_else(|| panic!("could not parse file count from output: {result}"));
    assert!(
        file_count >= 3,
        "file count should be >= 3 after writing 3 .md files, got {file_count}: {result}"
    );
}
