#![allow(dead_code)]

use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, ToolRegistry, db};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

// T-3.2: RTM FR-6
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub const REPAIR_FIXTURE: &str = r#"---
title: Test
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Using [[tools]] and [[coding-tool]] and [[tool system]].
"#;

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

pub fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write memory fixture");
}

pub fn read_memory_fixture(cfg: &Config, relative_path: &str) -> String {
    fs::read_to_string(cfg.memory_path.join(relative_path)).expect("read memory fixture")
}

// T-10.10: RTM FR-17
pub fn ensure_regular_vec_concepts_table(cfg: &Config) {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .execute_batch(
            r#"
            DROP TABLE IF EXISTS vec_concepts;
            CREATE TABLE vec_concepts (
                concept_name TEXT PRIMARY KEY,
                embedding BLOB
            );
            "#,
        )
        .expect("create vec_concepts table");
}

// T-10.10: RTM FR-17
pub fn insert_blob_embedding(cfg: &Config, concept_name: &str, embedding: &[f32]) {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let bytes: Vec<u8> = embedding
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, bytes],
        )
        .expect("insert blob embedding");
}

pub fn invoke_repair_concepts(payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke("RepairConcepts", &payload, OutputFormat::Markdown)
        .expect("invoke RepairConcepts")
}

pub fn try_invoke_repair_concepts(
    payload: Value,
) -> Result<String, maenifold_core::error::MaenifoldError> {
    ToolRegistry::global().try_invoke("RepairConcepts", &payload, OutputFormat::Markdown)
}

pub fn write_memory_via_registry(title: &str, content: &str) {
    let output = ToolRegistry::global()
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": title,
                "content": content
            }),
            OutputFormat::Json,
        )
        .expect("invoke WriteMemory");
    let value: Value = serde_json::from_str(&output).expect("write memory json");
    assert_eq!(
        value["success"], true,
        "seed write should succeed: {output}"
    );
}

pub fn seed_community_graph() {
    write_memory_via_registry(
        "Alpha Cluster 1",
        "This cluster links [[alpha]] with [[beta]] and [[gamma]].",
    );
    write_memory_via_registry(
        "Alpha Cluster 2",
        "This cluster links [[beta]] with [[gamma]].",
    );
    write_memory_via_registry("Omega Rho", "This cluster links [[omega]] with [[rho]].");
    write_memory_via_registry(
        "Omega Neighborhood",
        "This cluster links [[omega]], [[rho]], [[sigma]], and [[tau]].",
    );
    write_memory_via_registry(
        "Lambda Neighborhood",
        "This cluster links [[lambda]], [[rho]], [[sigma]], and [[tau]].",
    );
}

pub fn sync_graph() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("invoke Sync")
}

pub fn load_community_id(cfg: &Config, concept_name: &str) -> i64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
            [concept_name],
            |row| row.get::<_, i64>(0),
        )
        .expect("community id")
}

pub fn count_distinct_communities(cfg: &Config) -> i64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .query_row(
            "SELECT COUNT(DISTINCT community_id) FROM concept_communities",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("community count")
}

pub fn delete_community_assignments(cfg: &Config) {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .execute("DELETE FROM concept_communities", [])
        .expect("delete concept_communities rows");
}
