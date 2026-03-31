#![allow(dead_code)]

use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::{Connection, params};
use tempfile::TempDir;

// T-1.6: RTM NFR-1
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub(crate) struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

pub(crate) fn open_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

pub(crate) fn seed_graph(cfg: &Config) {
    let connection = open_connection(cfg);

    for concept in [
        "concept-a",
        "concept-b",
        "concept-c",
        "concept-d",
        "concept-e",
        "isolated-concept",
    ] {
        insert_concept(&connection, concept);
    }

    insert_edge(&connection, "concept-a", "concept-b", 2);
    insert_edge(&connection, "concept-a", "concept-c", 2);
    insert_edge(&connection, "concept-b", "concept-d", 2);
    insert_edge(&connection, "concept-c", "concept-e", 1);
}

fn insert_concept(connection: &Connection, name: &str) {
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
            params![name],
        )
        .expect("insert concept");
}

fn insert_edge(connection: &Connection, a: &str, b: &str, weight: i64) {
    let (concept_a, concept_b) = if a < b { (a, b) } else { (b, a) };
    let source_files = serde_json::to_string(
        &(0..weight)
            .map(|index| (format!("test://f{index}"), 1.0_f64))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )
    .expect("serialize source files");

    connection
        .execute(
            "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
             VALUES (?1, ?2, ?3, ?4)",
            params![concept_a, concept_b, weight, source_files],
        )
        .expect("insert edge");
}

// T-1.6: RTM NFR-1 — convert Err to "ERROR: {e}" so validation tests can assert
// on error strings instead of panicking on Result::Err.
pub(crate) fn visualize(cfg: &Config, payload: serde_json::Value) -> String {
    let context = markdown_context(cfg);
    search::visualize(&context, &payload).unwrap_or_else(|e| format!("ERROR: {e}"))
}
