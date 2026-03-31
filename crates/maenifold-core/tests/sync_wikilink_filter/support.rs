use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::path_to_uri;
use maenifold_core::search::{sync, sync_single_file};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub(crate) fn sync_one(cfg: &Config, path: &Path) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, path).expect("sync single file")
}

pub(crate) fn sync_all(cfg: &Config) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync(&context, &json!({})).expect("sync all")
}

pub(crate) fn create_test_markdown_file(
    cfg: &Config,
    file_name: &str,
    content: &str,
) -> (PathBuf, String) {
    let path = cfg
        .memory_path
        .join("tests")
        .join("sync-wikilink-filter")
        .join(file_name);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

pub(crate) fn get_mention_count(cfg: &Config, file_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, file_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query mention_count")
}

pub(crate) fn count_all_concepts(cfg: &Config) -> i64 {
    db_connection(cfg)
        .query_row("SELECT COUNT(*) FROM concepts", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("count concepts")
}

pub(crate) fn count_concept_graph_entries(cfg: &Config, concept_name: &str) -> i64 {
    db_connection(cfg)
        .query_row(
            "SELECT COUNT(*) FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1",
            params![concept_name],
            |row| row.get::<_, i64>(0),
        )
        .expect("count concept_graph entries")
}

pub(crate) fn write_wikilink_filter(cfg: &Config, filter_json: &str) {
    let filter_path = cfg.memory_path.join(".wikilink-filter.json");
    fs::write(&filter_path, filter_json).expect("write filter");
}

pub(crate) fn reset_database(cfg: &Config) {
    for suffix in ["", "-shm", "-wal"] {
        let path = PathBuf::from(format!("{}{}", cfg.database_path.display(), suffix));
        if path.exists() {
            fs::remove_file(&path).expect("remove db artifact");
        }
    }
    GraphDatabase::initialize_database(cfg).expect("db reinit");
}
