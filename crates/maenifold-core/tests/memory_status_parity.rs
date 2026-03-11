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

// T-13.4: RTM FR-7 — MemoryStatus output contains folder count.
#[test]
fn memory_status_contains_folder_count() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Create a subdirectory to verify folder counting.
    let sub = env.config.memory_path.join("test-folder");
    fs::create_dir_all(&sub).expect("create subdir");
    fs::write(sub.join("note.md"), "# Test note\n\nContent").expect("write file");

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("Total folders:"),
        "output should contain folder count field: {result}"
    );
    // Should have at least 1 folder (the one we just created).
    assert!(
        !result.contains("Total folders: 0"),
        "should count at least 1 folder: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus output contains total size in MB.
#[test]
fn memory_status_contains_total_size() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Write a file so total size > 0.
    fs::write(
        env.config.memory_path.join("size-test.md"),
        "# Size test\n\nSome content here for size measurement.",
    )
    .expect("write file");

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("Total size:"),
        "output should contain total size field: {result}"
    );
    assert!(
        result.contains("MB"),
        "total size should be in MB: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus output contains concept/relation/mention counts.
#[test]
fn memory_status_contains_graph_stats() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("### Graph Database"),
        "output should contain Graph Database section: {result}"
    );
    assert!(
        result.contains("- Concepts:"),
        "output should contain concept count: {result}"
    );
    assert!(
        result.contains("- Relations:"),
        "output should contain relation count: {result}"
    );
    assert!(
        result.contains("- Mentions:"),
        "output should contain mention count: {result}"
    );
}

// T-13.4: RTM FR-7 — MemoryStatus output contains DB file size in MB.
#[test]
fn memory_status_contains_db_size() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("Database size:"),
        "output should contain DB size field: {result}"
    );
    assert!(
        result.contains("MB"),
        "DB size should be in MB: {result}"
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

// T-13.4: RTM FR-7 — MemoryStatus file count reflects actual files.
#[test]
fn memory_status_file_count_reflects_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Write 3 files into memory root.
    for i in 0..3 {
        fs::write(
            env.config.memory_path.join(format!("file{i}.md")),
            format!("# File {i}\n\nContent"),
        )
        .expect("write file");
    }

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // The total file count includes both our 3 files and any assets initialized by the test env.
    // Just verify it shows "Total files:" with a non-zero value.
    assert!(
        result.contains("Total files:"),
        "output should contain file count: {result}"
    );
    assert!(
        !result.contains("Total files: 0"),
        "file count should not be 0 after adding files: {result}"
    );
}
