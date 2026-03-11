#![allow(dead_code)]

use maenifold_core::db;
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::from_str;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-12.7: RTM FR-14 - share concept-sync edge fixtures while splitting regression coverage by lifecycle theme.
pub fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

pub fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, content).expect("write fixture");
}

pub fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), maenifold_core::json::OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}

pub fn file_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

pub fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub fn set_last_indexed(cfg: &Config, file_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, value],
        )
        .expect("set last_indexed");
}

pub fn query_edge_rowid(cfg: &Config, a: &str, b: &str) -> Option<i64> {
    let (a, b) = normalize_edge(a, b);
    db_connection(cfg)
        .query_row(
            "SELECT rowid FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![a, b],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query rowid")
}

pub fn query_edge(cfg: &Config, a: &str, b: &str) -> Option<(f64, BTreeMap<String, f64>)> {
    let (a, b) = normalize_edge(a, b);
    db_connection(cfg)
        .query_row(
            "SELECT COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') \
             FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![a, b],
            |row| {
                let count = row.get::<_, f64>(0)?;
                let raw_files = row.get::<_, String>(1)?;
                let files = if raw_files.trim().is_empty() {
                    BTreeMap::new()
                } else {
                    from_str::<BTreeMap<String, f64>>(&raw_files).expect("parse source_files")
                };
                Ok((count, files))
            },
        )
        .optional()
        .expect("query edge")
}

pub fn normalize_edge<'a>(x: &'a str, y: &'a str) -> (&'a str, &'a str) {
    if x <= y { (x, y) } else { (y, x) }
}

pub fn initial_sectioned_content() -> &'static str {
    "# Test\n\n## Section One\n\n[[alpha]] [[beta]]\n\n## Section Two\n\n[[gamma]]\n"
}
