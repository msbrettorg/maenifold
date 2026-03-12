use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
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
    Config::reset_overrides();
}

#[test]
fn tool_registry_is_case_insensitive_and_exposes_contract_names() {
    let _guard = TEST_MUTEX.lock();
    let registry = ToolRegistry::global();
    assert!(registry.contains("WriteMemory"));
    assert!(registry.contains("writememory"));
    assert!(registry.contains("LISTASSETS"));
    assert!(registry.canonical_names().contains(&"ReadMcpResource"));
}

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
    for expected in [
        "file_content",
        "file_search",
        "concept_graph",
        "concept_communities",
        "sync_leader_lease",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
    Config::reset_overrides();
}

#[test]
fn assets_bootstrap_and_catalog_are_available() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let manager = AssetManager::new(&cfg);
    let catalog = manager.catalog().expect("catalog");
    assert!(!catalog.single_agent_workflows.is_empty());
    assert!(!catalog.roles.is_empty());
    assert!(!catalog.colors.is_empty());
    assert!(!catalog.perspectives.is_empty());
    let workflow = manager
        .read_resource("asset://workflows/deductive-reasoning")
        .expect("workflow");
    assert_eq!(
        workflow.get("id").and_then(|v| v.as_str()),
        Some("deductive-reasoning")
    );
    Config::reset_overrides();
}

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
            r#"{{"id":"{sentinel_id}","name":"T-10.17 Default Catalog Root","prompt":"sentinel"}} "#
        )
        .trim(),
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
