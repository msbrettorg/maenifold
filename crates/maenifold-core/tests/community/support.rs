// T-12.7: RTM NFR-1
#![allow(dead_code)]

use maenifold_core::{Config, GraphDatabase};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// T-W1-8: RTM NFR-2 — serialise all tests: they share a process-wide Config singleton.
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

// T-W1-8: RTM NFR-2
pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) db_path: PathBuf,
}

impl TestEnv {
    pub(crate) fn setup() -> Self {
        let tmp = TempDir::new().expect("tmp dir");
        Config::override_root(tmp.path());
        let db_path = tmp.path().join("memory.db");
        Config::set_database_path(&db_path);
        let cfg = Config::current();
        cfg.ensure_directories().expect("ensure dirs");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self { _tmp: tmp, db_path }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-W1-8: RTM NFR-2
pub(crate) fn open_conn(db_path: &Path) -> Connection {
    let conn = Connection::open(db_path).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .expect("pragmas");
    conn
}

// T-W1-8: RTM NFR-2
pub(crate) fn insert_concept(conn: &Connection, name: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params![name],
    )
    .expect("insert concept");
}

// T-W1-8: RTM NFR-2
pub(crate) fn insert_edge(conn: &Connection, a: &str, b: &str, weight: i64) {
    let (sorted_a, sorted_b) = if a < b { (a, b) } else { (b, a) };
    let entries: Vec<String> = (0..weight)
        .map(|i| format!("\"test://f{i}\": 1.0"))
        .collect();
    let source_files = format!("{{{}}}", entries.join(", "));
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph \
         (concept_a, concept_b, co_occurrence_count, source_files) \
         VALUES (?1, ?2, ?3, ?4)",
        params![sorted_a, sorted_b, weight, source_files],
    )
    .expect("insert edge");
}

// T-W1-8: RTM NFR-2 — insert the canonical test topology used by many tests.
pub(crate) fn insert_triangle_plus_isolated_pair(conn: &Connection) {
    for name in ["alpha", "beta", "gamma", "delta", "epsilon"] {
        insert_concept(conn, name);
    }
    insert_edge(conn, "alpha", "beta", 5);
    insert_edge(conn, "beta", "gamma", 5);
    insert_edge(conn, "alpha", "gamma", 5);
    insert_edge(conn, "delta", "epsilon", 5);
}

pub(crate) fn community_row_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM concept_communities", [], |r| r.get(0))
        .expect("count communities rows")
}

pub(crate) fn distinct_community_count(conn: &Connection) -> usize {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT community_id) FROM concept_communities",
            [],
            |r| r.get(0),
        )
        .expect("distinct count");
    count as usize
}

pub(crate) fn get_community_id(conn: &Connection, concept: &str) -> Option<i64> {
    conn.query_row(
        "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
        params![concept],
        |r| r.get(0),
    )
    .ok()
}

pub(crate) fn get_modularity(conn: &Connection) -> f64 {
    conn.query_row(
        "SELECT COALESCE(modularity, 0.0) FROM concept_communities LIMIT 1",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0.0)
}

pub(crate) fn get_resolution(conn: &Connection) -> f64 {
    conn.query_row(
        "SELECT COALESCE(resolution, 0.0) FROM concept_communities LIMIT 1",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0.0)
}
