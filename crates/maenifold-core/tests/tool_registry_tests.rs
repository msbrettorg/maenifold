use maenifold_core::error::MaenifoldError;
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::BTreeSet;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
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

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn new_registry_registers_all_expected_tool_names() {
    let registry = ToolRegistry::new();
    let expected = BTreeSet::from([
        "WriteMemory",
        "ReadMemory",
        "EditMemory",
        "DeleteMemory",
        "MoveMemory",
        "SearchMemories",
        "BuildContext",
        "Sync",
        "SequentialThinking",
        "Workflow",
        "RecentActivity",
        "MemoryStatus",
        "GetConfig",
        "GetHelp",
        "ListMemories",
        "UpdateAssets",
        "RepairConcepts",
        "AnalyzeConceptCorruption",
        "FindSimilarConcepts",
        "StartWatcher",
        "StopWatcher",
        "RunFullBenchmark",
        "Adopt",
        "AssumptionLedger",
        "ListAssets",
        "ReadMcpResource",
        "Visualize",
        "ExtractConceptsFromFile",
    ]);
    let actual = registry
        .canonical_names()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "registry should register every canonical tool"
    );
    for tool_name in &expected {
        assert!(
            registry.contains(tool_name),
            "tool '{tool_name}' should be registered"
        );
    }
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_with_valid_tool_name_returns_successful_result() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("GetConfig", json!({})).expect("GetConfig should succeed");

    assert!(
        result.contains("Maenifold Configuration"),
        "unexpected result: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_with_lowercase_alias_dispatches_same_as_canonical() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let canonical =
        invoke_markdown("GetConfig", json!({})).expect("canonical GetConfig should succeed");
    let alias = invoke_markdown("getconfig", json!({})).expect("alias getconfig should succeed");

    assert_eq!(
        alias, canonical,
        "alias dispatch should match canonical output"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_case_insensitive_lookup_always_resolves() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for name in ["GETCONFIG", "GetConfig", "getconfig", "gEtCoNfIg"] {
        let result = invoke_markdown(name, json!({})).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert!(
            result.contains(env.config.memory_path.to_string_lossy().as_ref()),
            "expected config output for '{name}', got: {result}"
        );
    }
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_with_unknown_tool_returns_unknown_tool_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let err = invoke_markdown("NonExistentTool_XYZ", json!({})).expect_err("should fail");

    match err {
        MaenifoldError::UnknownTool(name) => assert_eq!(name, "NonExistentTool_XYZ"),
        other => panic!("expected UnknownTool error, got: {other}"),
    }
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_with_empty_or_whitespace_name_returns_unknown_tool_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    for name in ["", "   "] {
        let err = invoke_markdown(name, json!({})).expect_err("blank name should fail");
        match err {
            MaenifoldError::UnknownTool(actual) => assert_eq!(actual, name),
            other => panic!("expected UnknownTool for '{name}', got: {other}"),
        }
    }
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_read_memory_with_missing_file_returns_not_found_message() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown(
        "ReadMemory",
        json!({"identifier":"memory://tool-registry-test-does-not-exist"}),
    )
    .expect("ReadMemory should return a rendered not-found response");

    let lower = result.to_ascii_lowercase();
    assert!(lower.contains("not found") || lower.contains("error"));
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_get_config_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = invoke_markdown("GetConfig", json!({})).expect("GetConfig should succeed");

    assert!(
        result.contains(env.config.memory_path.to_string_lossy().as_ref()),
        "config output should include memory path info: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_memory_status_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    assert!(
        result.contains("\"fileCount\""),
        "unexpected result: {result}"
    );
    assert!(
        result.contains(env.config.database_path.to_string_lossy().as_ref()),
        "memory status should include database path: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_get_help_with_tool_name_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result =
        invoke_markdown("GetHelp", json!({"toolName":"GetHelp"})).expect("GetHelp should work");

    assert!(
        result.to_ascii_lowercase().contains("gethelp"),
        "unexpected help output: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_list_memories_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_markdown("ListMemories", json!({})).expect("ListMemories should succeed");

    assert!(
        result.contains("# Directory:") || result.contains("### Files"),
        "unexpected list output: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn constructing_registry_twice_produces_the_same_registrations() {
    let first = ToolRegistry::new();
    let second = ToolRegistry::new();

    assert_eq!(first.canonical_names(), second.canonical_names());
    assert!(first.contains("GetConfig"));
    assert!(second.contains("GetConfig"));
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn global_registry_invocation_works_without_explicit_setup_call() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result =
        ToolRegistry::global().try_invoke("MemoryStatus", &json!({}), OutputFormat::Markdown);

    let output = result.expect("global registry should lazily initialize");
    assert!(
        output.contains("\"fileCount\""),
        "unexpected output: {output}"
    );
}
