// NFR-2, NFR-3: Integration smoke tests for core subsystem contracts.
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

// NFR-2, NFR-3: Config path contract — only dedicated Config contract test.
#[test]
fn config_honors_override_paths() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    assert!(cfg.memory_path.ends_with("memory"));
    assert!(cfg.assets_path.ends_with("assets"));
    assert_eq!(
        cfg.database_path.file_name().and_then(|s| s.to_str()),
        Some("memory.db")
    );
    // P3: Verify ensure_directories() actually created the filesystem.
    assert!(cfg.memory_path.is_dir(), "memory dir should exist on disk");
    assert!(cfg.assets_path.is_dir(), "assets dir should exist on disk");
    assert!(
        cfg.database_path.parent().unwrap().is_dir(),
        "database parent dir should exist on disk"
    );
    Config::reset_overrides();
}

// Test 2 REMOVED: tool_registry_is_case_insensitive_and_exposes_contract_names
// Fully subsumed by tool_registry_tests::new_registry_registers_all_expected_tool_names
// and tool_registry_tests::try_invoke_case_insensitive_lookup_always_resolves.

// FR-22 (T-16.1), NFR-2, NFR-3: Schema contract — only sqlite_master enumeration test.
// The query returns virtual FTS5 tables as type='table' in practice; this is an
// implicit SQLite behaviour, not a bug.
#[test]
fn graph_database_creates_shared_schema_tables() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    GraphDatabase::initialize_database(&cfg).expect("db init");
    let conn = Connection::open(&cfg.database_path).expect("open db");
    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','view')")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<std::result::Result<_, _>>()
        .expect("collect");
    // P0: All 7 core schema objects from schema.rs must be present.
    for expected in [
        "file_content",
        "file_search",
        "concepts",
        "concept_mentions",
        "concept_graph",
        "concept_communities",
        "sync_leader_lease",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing table/view: {expected} (found: {names:?})"
        );
    }
    Config::reset_overrides();
}

// NFR-2, NFR-3: Asset bootstrap contract — only read_resource() URI routing test.
#[test]
fn assets_bootstrap_and_catalog_are_available() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let manager = AssetManager::new(&cfg);
    let catalog = manager.catalog().expect("catalog");
    // P2: Minimum-count assertions instead of bare !is_empty().
    assert!(
        catalog.single_agent_workflows.len() >= 4,
        "expected at least 4 single-agent workflows, got {}",
        catalog.single_agent_workflows.len()
    );
    assert!(
        catalog.roles.len() >= 4,
        "expected at least 4 roles, got {}",
        catalog.roles.len()
    );
    assert!(
        catalog.colors.len() >= 4,
        "expected at least 4 colors, got {}",
        catalog.colors.len()
    );
    assert!(
        catalog.perspectives.len() >= 4,
        "expected at least 4 perspectives, got {}",
        catalog.perspectives.len()
    );
    // P2: Derive URI dynamically from catalog instead of hardcoding a workflow name.
    let first_workflow = catalog
        .single_agent_workflows
        .first()
        .expect("at least one single-agent workflow");
    let first_id = first_workflow
        .get("id")
        .and_then(|v| v.as_str())
        .expect("workflow should have an id field");
    let uri = format!("asset://workflows/{first_id}");
    let workflow = manager
        .read_resource(&uri)
        .unwrap_or_else(|e| panic!("read_resource({uri}) failed: {e}"));
    assert_eq!(
        workflow.get("id").and_then(|v| v.as_str()),
        Some(first_id),
        "read_resource URI routing should round-trip the workflow id"
    );
    Config::reset_overrides();
}

// NFR-2, NFR-3: Path-binding contract — unique sentinel-file proof.
#[test]
fn catalog_uses_current_config_assets_path_by_default() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let sentinel_id = "t-10-17-default-catalog-root";
    let sentinel_path = cfg
        .assets_path
        .join("roles")
        .join(format!("{sentinel_id}.json"));
    fs::write(
        &sentinel_path,
        format!(
            r#"{{"id":"{sentinel_id}","name":"T-10.17 Default Catalog Root","prompt":"sentinel"}}"#
        ),
    )
    .expect("write sentinel role");

    let manager = AssetManager::new(&cfg);
    let catalog = manager.catalog().expect("catalog");

    assert!(
        catalog
            .roles
            .iter()
            .any(|role| { role.get("id").and_then(|value| value.as_str()) == Some(sentinel_id) }),
        "catalog should read roles from Config::current().assets_path by default"
    );

    Config::reset_overrides();
}
