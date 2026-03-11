use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::fs;
use tempfile::TempDir;

// T-12.7: RTM NFR-1 - Share benchmark boundary fixtures across split themed tests.
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

pub fn invoke_benchmark(payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke("RunFullBenchmark", &payload, OutputFormat::Markdown)
        .expect("invoke RunFullBenchmark")
}

pub fn invoke_benchmark_result(
    payload: serde_json::Value,
) -> Result<String, maenifold_core::MaenifoldError> {
    ToolRegistry::global().try_invoke("RunFullBenchmark", &payload, OutputFormat::Markdown)
}

pub fn seed_file_content_bulk(conn: &Connection, count: usize) {
    for i in 0..count {
        conn.execute(
            "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("memory://benchmark-test/file-{i}"),
                format!("Benchmark File {i}"),
                format!(
                    "This file contains content about machine learning algorithms and \
                     knowledge graph structures for semantic search testing. \
                     Unique token: token_{i}."
                ),
                format!("md5-bench-{i}"),
                100i64 + i as i64,
                "2026-06-01T00:00:00Z",
                "active"
            ],
        )
        .unwrap_or_else(|e| panic!("seed file_content row {i}: {e}"));
    }
}

pub fn seed_graph_data(conn: &Connection, hub_count: usize, edges_per_hub: usize) {
    for h in 0..hub_count {
        let hub = format!("hub-concept-{h}");
        conn.execute(
            "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
            params![hub],
        )
        .expect("insert hub concept");
        for e in 0..edges_per_hub {
            let leaf = format!("leaf-{h}-{e}");
            conn.execute(
                "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
                params![leaf],
            )
            .expect("insert leaf concept");
            conn.execute(
                "INSERT OR IGNORE INTO concept_graph (concept_a, concept_b, co_occurrence_count)
                 VALUES (?1, ?2, 1)",
                params![hub, leaf],
            )
            .expect("insert edge");
        }
    }
}

pub fn write_memory_file(cfg: &Config, relative: &str, content: &str) {
    let path = cfg.memory_path.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write markdown");
}
