use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn markdown_ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn json_ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }

    fn filter_path(&self) -> PathBuf {
        self.cfg.memory_path.join(".wikilink-filter.json")
    }

    fn test_folder_path(&self) -> PathBuf {
        self.cfg
            .memory_path
            .join("wikilink-filter-integration-tests")
    }
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
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");

    let env = TestEnv { _tmp: tmp, cfg };
    fs::create_dir_all(env.test_folder_path()).expect("create integration test folder");
    write_filter_file(&env);
    env
}

fn write_filter_file(env: &TestEnv) {
    // T-10.16: RTM FR-19 / RTM-102 / RTM-103 / RTM-104 seeds the shared-root JSON WikiLink filter for integration coverage.
    fs::write(
        env.filter_path(),
        json!({
            "tool": "Generic hub - too connected",
            "agent": "Generic hub - too connected"
        })
        .to_string(),
    )
    .expect("write wikilink filter");
}

fn remove_filter_file(env: &TestEnv) {
    let path = env.filter_path();
    if path.exists() {
        fs::remove_file(path).expect("remove wikilink filter");
    }
}

// T-2.4: RTM FR-11.2 / NFR-11.2.1
#[test]
fn write_memory_with_filtered_concept_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.markdown_ctx(),
        &json!({
            "title": "Test",
            "content": "About [[tool]] and [[react]]"
        }),
    )
    .expect("write memory");

    assert!(result.contains("ERROR"), "expected error marker: {result}");
    // PARITY-WIKIFILTER: RTM NFR-1 — updated message to match C# text.
    assert!(
        result.contains("WikiLinks that are excluded"),
        "expected exclusion message: {result}"
    );
    // T-2.4: Verify the filtered concept name appears in the error output.
    assert!(
        result.contains("[[tool]]"),
        "error must name the filtered concept: {result}"
    );
    // T-2.4: Blocked write must not create a file on disk.
    assert!(
        !result.starts_with("Created memory FILE:"),
        "blocked write should not create a file: {result}"
    );
}

// T-2.4: RTM FR-11.2 / NFR-11.2.1
#[test]
fn write_memory_mixed_filtered_and_valid_still_blocked() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.markdown_ctx(),
        &json!({
            "title": "Test Mixed",
            "content": "About [[tool]] and [[react]] and [[agent]]"
        }),
    )
    .expect("write memory");

    assert!(result.contains("ERROR"), "unexpected: {result}");
    assert!(result.contains("[[tool]]"), "unexpected: {result}");
    assert!(result.contains("[[agent]]"), "unexpected: {result}");
    // T-2.4: Unfiltered concept must NOT appear in the error output.
    assert!(
        !result.contains("[[react]]"),
        "unfiltered concept must not appear in error: {result}"
    );
    assert!(
        !result.starts_with("Created memory FILE:"),
        "blocked write should not create a file: {result}"
    );
}

// T-2.4: RTM FR-11.2 / NFR-11.2.1
#[test]
fn write_memory_without_filter_file_allows_all() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    remove_filter_file(&env);

    let result = memory::write_memory(
        &env.markdown_ctx(),
        &json!({
            "title": "Filter Removed Test",
            "content": "About [[tool]] and [[react]]",
            "folder": "wikilink-filter-integration-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
}

// T-2.4: RTM FR-11.2 / NFR-11.2.2
#[test]
fn edit_memory_with_filtered_concept_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    remove_filter_file(&env);

    let write_result = memory::write_memory(
        &env.markdown_ctx(),
        &json!({
            "title": "Edit Filter Test",
            "content": "Original content with [[react]]",
            "folder": "wikilink-filter-integration-tests"
        }),
    )
    .expect("seed write memory");
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    write_filter_file(&env);

    let edit_result = memory::edit_memory(
        &env.markdown_ctx(),
        &json!({
            "identifier": "memory://wikilink-filter-integration-tests/edit-filter-test",
            "operation": "append",
            "content": "New content with [[tool]]"
        }),
    )
    .expect("edit memory");

    assert!(edit_result.contains("ERROR"), "unexpected: {edit_result}");
    // PARITY-WIKIFILTER: RTM NFR-1 — updated message to match C# text.
    assert!(
        edit_result.contains("WikiLinks that are excluded"),
        "unexpected: {edit_result}"
    );
}

// T-2.4: RTM FR-11.2 / NFR-11.2.3
#[test]
fn write_memory_filtered_concept_json_mode_returns_structured_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.json_ctx(),
        &json!({
            "title": "Json Filter Test",
            "content": "About [[tool]] and [[react]]"
        }),
    )
    .expect("write memory");

    let parsed: Value = serde_json::from_str(&result).expect("json result");
    assert_eq!(parsed["success"], false, "unexpected json: {result}");
    assert_eq!(
        parsed["error"]["code"], "FILTERED_WIKILINKS",
        "unexpected error code: {result}"
    );
    // T-2.4: Verify blocked array names the filtered concept with its reason.
    let blocked = parsed["error"]["details"]["blocked"]
        .as_array()
        .expect("blocked must be an array");
    assert_eq!(
        blocked.len(),
        1,
        "exactly one filtered concept expected: {result}"
    );
    assert_eq!(
        blocked[0]["concept"], "tool",
        "must name the filtered concept: {result}"
    );
    assert_eq!(
        blocked[0]["reason"], "Generic hub - too connected",
        "must include the filter reason: {result}"
    );
}

// T-2.4: RTM FR-11.2 / NFR-11.2.2 / NFR-11.2.3
#[test]
fn edit_memory_filtered_concept_json_mode_returns_structured_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    remove_filter_file(&env);

    // Seed a file without the filter active.
    let write_result = memory::write_memory(
        &env.json_ctx(),
        &json!({
            "title": "Edit Json Filter Test",
            "content": "Original content with [[react]]",
            "folder": "wikilink-filter-integration-tests"
        }),
    )
    .expect("seed write memory");
    let write_parsed: Value = serde_json::from_str(&write_result).expect("json seed");
    assert_eq!(
        write_parsed["success"], true,
        "seed write should succeed: {write_result}"
    );

    // Re-enable the filter, then edit with a filtered concept.
    write_filter_file(&env);

    let edit_result = memory::edit_memory(
        &env.json_ctx(),
        &json!({
            "identifier": "memory://wikilink-filter-integration-tests/edit-json-filter-test",
            "operation": "append",
            "content": "New content with [[tool]]"
        }),
    )
    .expect("edit memory");

    let parsed: Value = serde_json::from_str(&edit_result).expect("json edit result");
    assert_eq!(
        parsed["success"], false,
        "edit with filtered concept should fail: {edit_result}"
    );
    assert_eq!(
        parsed["error"]["code"], "FILTERED_WIKILINKS",
        "unexpected error code: {edit_result}"
    );
    // T-2.4: Verify blocked array in JSON edit_memory path.
    let blocked = parsed["error"]["details"]["blocked"]
        .as_array()
        .expect("blocked must be an array");
    assert_eq!(
        blocked.len(),
        1,
        "exactly one filtered concept expected: {edit_result}"
    );
    assert_eq!(
        blocked[0]["concept"], "tool",
        "must name the filtered concept: {edit_result}"
    );
}
