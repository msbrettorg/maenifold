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

    let identifier = "memory://tool-registry-test-does-not-exist";
    let result = invoke_markdown("ReadMemory", json!({"identifier": identifier}))
        .expect("ReadMemory should return a rendered not-found response, not an Err");

    // The rendered response must reference the identifier the caller asked for,
    // proving the tool resolved the URI rather than returning a generic error.
    assert!(
        result.contains(identifier) || result.contains("tool-registry-test-does-not-exist"),
        "not-found response should reference the requested identifier: {result}"
    );
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
    let _env = isolated_env();

    let result = invoke_markdown("MemoryStatus", json!({})).expect("MemoryStatus should succeed");

    // T-13.4: RTM FR-7 — MemoryStatus now returns C#-parity markdown with file/folder/size stats.
    assert!(
        result.contains("# Memory System Status"),
        "should contain header: {result}"
    );
    assert!(
        result.contains("Total files:"),
        "should contain file count: {result}"
    );
    assert!(
        result.contains("Total folders:"),
        "should contain folder count: {result}"
    );
    assert!(
        result.contains("Total size:"),
        "should contain total size: {result}"
    );
    assert!(
        result.contains("### Graph Database"),
        "should contain graph database section: {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_get_help_with_tool_name_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result =
        invoke_markdown("GetHelp", json!({"toolName":"GetHelp"})).expect("GetHelp should work");

    // Assert structural markers from the help format, not just tool name presence.
    // A meaningful help response must contain parameter documentation or a description section.
    assert!(
        result.contains("## Parameters") || result.contains("Description"),
        "help output should contain structural markers (## Parameters or Description): {result}"
    );
}

// T-0.2: RTM FR-1 / FR-2 / FR-3 / FR-4 / FR-5 / FR-6
#[test]
fn try_invoke_list_memories_returns_non_null_result() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Seed a file so the listing has observable, test-specific content.
    let sentinel = env.config.memory_path.join("tool-registry-sentinel.md");
    std::fs::write(&sentinel, "# Sentinel\n\nCreated by tool_registry_tests.\n")
        .expect("write sentinel file");

    let result = invoke_markdown("ListMemories", json!({})).expect("ListMemories should succeed");

    // Assert the listing contains the sentinel filename, proving ListMemories
    // reads from the isolated test environment, not just any directory.
    assert!(
        result.contains("tool-registry-sentinel"),
        "list output should contain the sentinel file name: {result}"
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
    // T-13.4: RTM FR-7 — MemoryStatus returns markdown, not JSON.
    // Note: the MemoryStatus content assertion is incidental — the test's
    // distinct value is proving the global singleton path works without
    // a prior explicit ToolRegistry::new() call.
    assert!(
        output.contains("# Memory System Status"),
        "unexpected output: {output}"
    );
}

// T-0.2: RTM FR-1 — ToolRegistry::contains resolves aliases, not only canonical names
#[test]
fn registry_contains_resolves_aliases_not_only_canonical_names() {
    let registry = ToolRegistry::new();

    // Canonical name should work
    assert!(
        registry.contains("GetConfig"),
        "canonical name 'GetConfig' should be found"
    );
    // Lowercase alias should also resolve
    assert!(
        registry.contains("getconfig"),
        "lowercase alias 'getconfig' should be found via case-insensitive lookup"
    );
    // Mixed-case alias should also resolve
    assert!(
        registry.contains("GETCONFIG"),
        "uppercase alias 'GETCONFIG' should be found via case-insensitive lookup"
    );
    // Non-GetConfig tool alias to confirm aliasing isn't a GetConfig-only special case
    assert!(
        registry.contains("writememory"),
        "lowercase alias 'writememory' should be found via case-insensitive lookup"
    );
}
