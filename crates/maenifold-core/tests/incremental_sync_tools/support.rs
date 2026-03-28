#![allow(dead_code, unused_imports)]

use maenifold_core::db;
use maenifold_core::json::OutputFormat;
// T-12.6: RTM NFR-1 - Re-export split watcher helpers directly for regression tests.
pub use maenifold_core::markdown::{normalize_concept, path_to_uri};
pub use maenifold_core::sync::watchers::{
    changes_since_last_optimize, format_watcher_error_message, process_path_for_test,
    process_pending_paths_for_test, reset_optimize_counter, start_watcher, stop_watcher,
    watcher_is_running,
};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// T-12.7: RTM NFR-1 - Share incremental sync fixtures across split themed tests.
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        // T-15.21: RTM NFR-2 — Panic-safe cleanup: reset config overrides, the global
        // optimize counter, and the threshold env var so no state leaks between tests.
        Config::reset_overrides();
        reset_optimize_counter();
        unsafe { std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY") };
    }
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

pub fn context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

pub fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub fn create_test_file(cfg: &Config, subfolder: &str, content: &str) -> (PathBuf, String) {
    let folder = cfg.memory_path.join(subfolder);
    fs::create_dir_all(&folder).expect("mkdir");
    let file_path = folder.join(format!("note-{}.md", uuid::Uuid::new_v4().simple()));
    fs::write(&file_path, content).expect("write fixture");
    let uri = path_to_uri(&file_path, &cfg.memory_path).expect("memory uri");
    (file_path, uri)
}

pub fn get_mention_count(cfg: &Config, memory_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("mention count")
}

pub fn get_file_row(cfg: &Config, memory_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_path FROM file_content WHERE file_path = ?1",
            [memory_uri],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("file row")
}

pub fn get_file_md5(cfg: &Config, memory_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_md5 FROM file_content WHERE file_path = ?1",
            [memory_uri],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("file md5")
}

pub fn set_last_indexed(cfg: &Config, memory_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![memory_uri, value],
        )
        .expect("set last_indexed");
}

pub fn count_rows_like(cfg: &Config, table: &str, pattern: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE file_path LIKE ?1");
    db_connection(cfg)
        .query_row(&sql, [pattern], |row| row.get::<_, i64>(0))
        .expect("row count")
}
