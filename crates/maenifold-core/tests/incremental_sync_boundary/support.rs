#![allow(dead_code)]

// T-12.1: RTM NFR-1
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{Connection, params};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock()
}

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

pub fn file_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

pub fn baseline_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("baseline sync")
}

pub fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}

pub fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub fn fetch_last_indexed(cfg: &Config, file_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            [file_uri],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

pub fn fetch_file_md5(cfg: &Config, file_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_md5 FROM file_content WHERE file_path = ?1",
            [file_uri],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

pub fn set_last_indexed(cfg: &Config, file_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, value],
        )
        .expect("set last_indexed");
}

pub fn concept_names(cfg: &Config) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare("SELECT concept_name FROM concepts ORDER BY concept_name")
        .expect("prepare concepts");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query concepts")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concepts")
}

pub fn file_concepts(cfg: &Config, file_uri: &str) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare(
            "SELECT concept_name FROM concept_mentions WHERE source_file = ?1 ORDER BY concept_name",
        )
        .expect("prepare concept_mentions");
    statement
        .query_map([file_uri], |row| row.get::<_, String>(0))
        .expect("query concept_mentions")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concept_mentions")
}

pub fn concept_sources(cfg: &Config, concept_name: &str) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare(
            "SELECT source_file FROM concept_mentions WHERE concept_name = ?1 ORDER BY source_file",
        )
        .expect("prepare concept source query");
    statement
        .query_map([concept_name], |row| row.get::<_, String>(0))
        .expect("query concept sources")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concept sources")
}
