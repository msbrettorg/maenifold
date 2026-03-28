#![allow(dead_code, unused_imports)]

use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::repair;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// T-12.6: RTM NFR-1 — shared support keeps split vector regression tests aligned.
pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(crate) const TEST_FOLDER: &str = "vector-tools-tests";

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl TestEnv {
    pub(crate) fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn write_memory_fixture(env: &TestEnv, relative_path: &str, content: &str) {
    let path = env.cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture dirs");
    fs::write(path, content).expect("fixture write");
}

// T-14.9: RTM FR-6 — Load the vec0 extension on a connection so that virtual
// tables created by GraphDatabase::initialize_database() can be dropped.  If the
// dylib is unavailable the virtual table was never created, so DROP is a no-op.
fn try_load_vec0_extension(conn: &rusqlite::Connection, assets_path: &Path) {
    let (platform_dir, filename) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("osx-arm64", "vec0.dylib"),
        ("macos", "x86_64") => ("osx-x64", "vec0.dylib"),
        ("linux", "x86_64") => ("linux-x64", "vec0.so"),
        ("linux", "aarch64") => ("linux-arm64", "vec0.so"),
        ("windows", "x86_64") => ("win-x64", "vec0.dll"),
        _ => return,
    };
    let path = assets_path.join("native").join(platform_dir).join(filename);
    if path.exists() {
        unsafe {
            let _ = conn.load_extension_enable();
            let _ = conn.load_extension(&path, None::<&str>);
            let _ = conn.load_extension_disable();
        }
    }
}

// T-14.9: RTM FR-6 — Replaces the vec0 virtual table created by initialize_database()
// with a plain table so tests can insert raw embedding blobs directly.  The vec0
// extension must be loaded first because SQLite cannot DROP a virtual table whose
// module is not present.
pub(crate) fn ensure_regular_vec_concepts_table(env: &TestEnv) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    try_load_vec0_extension(&connection, &env.cfg.assets_path);
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

pub(crate) fn insert_blob_embedding(env: &TestEnv, concept_name: &str, embedding: &[f32]) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, encode_blob_embedding(embedding)],
        )
        .expect("insert blob embedding");
}

pub(crate) fn insert_json_embedding(env: &TestEnv, concept_name: &str, embedding: &[f32]) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    let payload = serde_json::to_string(embedding).expect("serialize embedding");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, payload],
        )
        .expect("insert json embedding");
}

pub(crate) fn insert_text_embedding(env: &TestEnv, concept_name: &str, embedding: &str) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, embedding],
        )
        .expect("insert text embedding");
}

pub(crate) fn insert_null_embedding(env: &TestEnv, concept_name: &str) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, NULL)",
            params![concept_name],
        )
        .expect("insert null embedding");
}

pub(crate) fn invoke_repair_concepts(
    env: &TestEnv,
    payload: serde_json::Value,
) -> maenifold_core::Result<String> {
    repair::repair_concepts(&env.ctx_markdown(), &payload)
}

pub(crate) fn deterministic_embedding(seed: f32) -> Vec<f32> {
    (0..384)
        .map(|index| {
            let step = ((index % 11) as f32 - 5.0) / 16.0;
            seed + step
        })
        .collect()
}

pub(crate) fn exact_precision_embedding() -> Vec<f32> {
    (0..384)
        .map(|index| match index % 4 {
            0 => 0.0,
            1 => 0.5,
            2 => 1.0,
            _ => -0.5,
        })
        .collect()
}

pub(crate) fn encode_blob_embedding(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

pub(crate) fn assert_embeddings_match(left: &[f32], right: &[f32]) {
    assert_eq!(left.len(), right.len(), "embedding dimensions must match");
    for (index, (left_value, right_value)) in left.iter().zip(right.iter()).enumerate() {
        assert!(
            (left_value - right_value).abs() <= f32::EPSILON,
            "embeddings diverged at dimension {index}: {left_value} vs {right_value}"
        );
    }
}
