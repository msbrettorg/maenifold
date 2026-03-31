// T-12.0: RTM NFR-1
#![allow(dead_code)]

use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use parking_lot::MutexGuard;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct TestEnv {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn isolated() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&cfg).expect("assets");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self {
            _guard: guard,
            _tmp: tmp,
            cfg,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    pub fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

pub fn open_db(cfg: &Config) -> Connection {
    let connection = Connection::open(&cfg.database_path).expect("open db");
    connection
        .execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .expect("pragmas");
    connection
}

pub fn write_memory_file(cfg: &Config, relative_path: &str, content: &str) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
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

pub fn invoke_build_context_json(concept_name: &str) -> Value {
    let raw = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": concept_name, "depth": 1, "maxEntities": 20 }),
            OutputFormat::Json,
        )
        .expect("invoke build context");
    serde_json::from_str(&raw).expect("build context json")
}

pub fn insert_concept(conn: &Connection, name: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params![name],
    )
    .expect("insert concept");
}

pub fn insert_edge_with_proximity(conn: &Connection, a: &str, b: &str, uri: &str, weight: f64) {
    let (left, right) = if a < b { (a, b) } else { (b, a) };
    let source_files = serde_json::to_string(&json!({ uri: weight })).expect("source_files json");
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
         VALUES (?1, ?2, 1, ?3)",
        params![left, right, source_files],
    )
    .expect("insert edge");
}

pub fn insert_edge_raw_source_files(
    conn: &Connection,
    a: &str,
    b: &str,
    co_occurrence_count: f64,
    source_files_json: &str,
) {
    let (left, right) = if a < b { (a, b) } else { (b, a) };
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
         VALUES (?1, ?2, ?3, ?4)",
        params![left, right, co_occurrence_count, source_files_json],
    )
    .expect("insert raw edge");
}

pub fn relation_source_files(conn: &Connection, concept_a: &str, concept_b: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT source_files FROM concept_graph WHERE (concept_a = ?1 AND concept_b = ?2) OR (concept_a = ?2 AND concept_b = ?1)",
            params![concept_a, concept_b],
            |row| row.get(0),
        )
        .expect("relation source_files");
    serde_json::from_str(&raw).expect("source_files should be valid json")
}

pub fn relation_weight_for_uri(
    conn: &Connection,
    concept_a: &str,
    concept_b: &str,
    uri: &str,
) -> f64 {
    relation_source_files(conn, concept_a, concept_b)[uri]
        .as_f64()
        .expect("relation weight should be numeric")
}

pub fn relation_count(conn: &Connection, concept_a: &str, concept_b: &str) -> Option<f64> {
    conn.query_row(
        "SELECT co_occurrence_count FROM concept_graph WHERE (concept_a = ?1 AND concept_b = ?2) OR (concept_a = ?2 AND concept_b = ?1)",
        params![concept_a, concept_b],
        |row| row.get(0),
    )
    .optional()
    .expect("read relation count")
}

pub fn relation_exists(conn: &Connection, concept_a: &str, concept_b: &str) -> bool {
    relation_count(conn, concept_a, concept_b).is_some()
}

pub fn direct_relation<'a>(payload: &'a Value, name: &str) -> &'a Value {
    payload["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array")
        .iter()
        .find(|relation| relation["Name"].as_str() == Some(name))
        .expect("relation should exist")
}

pub fn community_id(conn: &Connection, concept_name: &str) -> i64 {
    conn.query_row(
        "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
        params![concept_name],
        |row| row.get(0),
    )
    .expect("community id")
}

pub fn insert_file_content(
    conn: &Connection,
    uri: &str,
    created: &str,
    last_accessed: Option<&str>,
) {
    conn.execute(
        "INSERT OR REPLACE INTO file_content (file_path, title, content, created, last_accessed) \
         VALUES (?1, ?2, '', ?3, ?4)",
        params![uri, uri, created, last_accessed],
    )
    .expect("insert file_content");
}

pub fn assert_approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "expected {expected:.10}, got {actual:.10}"
    );
}

#[cfg(unix)]
pub fn capture_stderr<F, T>(operation: F) -> (T, String)
where
    F: FnOnce() -> T,
{
    use std::fs::File;
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd};

    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(src: i32, dest: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let stderr = std::io::stderr();
    let stderr_fd = stderr.as_raw_fd();
    let mut pipe_fds = [0_i32; 2];

    let pipe_result = unsafe { pipe(pipe_fds.as_mut_ptr()) };
    assert_eq!(pipe_result, 0, "pipe failed");

    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");

    let redirect_result = unsafe { dup2(pipe_fds[1], stderr_fd) };
    assert!(redirect_result >= 0, "dup2 failed");

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    assert!(
        unsafe { dup2(saved_stderr, stderr_fd) } >= 0,
        "restore dup2 failed"
    );
    unsafe {
        close(pipe_fds[1]);
        close(saved_stderr);
    }

    let mut output = String::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}
