#![allow(dead_code, unused_imports)]

// T-12.0: RTM NFR-1
use chrono::{Duration, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::memory::compute_decay_weight;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use parking_lot::MutexGuard;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    pub fn with_defaults() -> Self {
        let keys = [
            "MAENIFOLD_DECAY_FUNCTION",
            "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
            "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
            "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
            "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
        ];

        let previous = keys
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect::<Vec<_>>();

        // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
        // so these process-wide environment mutations are serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", "30");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", "7");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", "14");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", "28");
        }

        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe { std::env::set_var(key, value) }
            } else {
                // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe { std::env::remove_var(key) }
            }
        }
    }
}

impl TestEnv {
    pub fn isolated() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&cfg).expect("assets");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self { _tmp: tmp, cfg }
    }
}

pub fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock()
}

pub fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

pub fn write_memory_file(cfg: &Config, relative_path: &str, content: &str) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(file_path.parent().expect("parent directory")).expect("create parent dirs");
    fs::write(&file_path, content).expect("write memory file");
    memory_uri_from_relative(relative_path)
}

pub fn memory_uri_from_relative(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

pub fn invoke_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("invoke sync")
}

pub fn invoke_search_memories_json(query: &str) -> Value {
    let raw = ToolRegistry::global()
        .try_invoke(
            "SearchMemories",
            &json!({ "query": query, "mode": "Hybrid", "pageSize": 10, "page": 1 }),
            OutputFormat::Json,
        )
        .expect("invoke search memories");
    serde_json::from_str(&raw).expect("search json")
}

pub fn relation_source_files(conn: &Connection, concept_a: &str, concept_b: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT source_files
             FROM concept_graph
             WHERE (concept_a = ?1 AND concept_b = ?2)
                OR (concept_a = ?2 AND concept_b = ?1)",
            params![concept_a, concept_b],
            |row| row.get(0),
        )
        .expect("relation source_files");
    serde_json::from_str(&raw).expect("source_files should be valid JSON")
}

pub fn relation_weight_for_uri(
    conn: &Connection,
    concept_a: &str,
    concept_b: &str,
    uri: &str,
) -> f64 {
    relation_source_files(conn, concept_a, concept_b)[uri]
        .as_f64()
        .expect("source_files entry should be numeric")
}

pub fn set_last_accessed(cfg: &Config, uri: &str, last_accessed: &str) {
    open_db(cfg)
        .execute(
            "UPDATE file_content SET last_accessed = ?1 WHERE file_path = ?2",
            params![last_accessed, uri],
        )
        .expect("update last_accessed");
}

pub fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

pub fn assert_weight_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected weight {expected}, got {actual}"
    );
}

pub fn decay_weight(path: &str, last_accessed: Option<&str>, cfg: &Config) -> f64 {
    compute_decay_weight(path, last_accessed, cfg, None)
}
