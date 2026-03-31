#![allow(dead_code, unused_imports)]

use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search::{sync, sync_single_file};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{from_str, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Guards Config::override_root() process-global state, not DB or filesystem contention.
// Each test has its own TempDir and DB path, but override_root() mutates a process-global
// config object. Without this mutex, concurrent test threads would clobber each other's
// config even though their TempDirs are isolated.
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
        .join("sync-unification")
        .join(file_name);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

#[derive(Debug, Clone)]
pub(crate) struct FileContentRow {
    pub(crate) last_indexed: Option<String>,
    pub(crate) file_md5: Option<String>,
    pub(crate) file_size: Option<i64>,
}

pub(crate) fn get_file_content_row(cfg: &Config, file_uri: &str) -> Option<FileContentRow> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = ?1",
            [file_uri],
            |row| {
                Ok(FileContentRow {
                    last_indexed: row.get(0)?,
                    file_md5: row.get(1)?,
                    file_size: row.get(2)?,
                })
            },
        )
        .optional()
        .expect("query file_content")
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

pub(crate) fn tamper_delete_mentions(cfg: &Config, file_uri: &str) {
    db_connection(cfg)
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![file_uri],
        )
        .expect("delete mentions");
}

pub(crate) fn parse_rfc3339_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("parse rfc3339")
        .with_timezone(&Utc)
}

pub(crate) fn set_file_mtime_utc(path: &Path, instant: DateTime<Utc>) {
    set_file_mtime(
        path,
        FileTime::from_unix_time(instant.timestamp(), instant.timestamp_subsec_nanos()),
    )
    .expect("set mtime");
}

pub(crate) fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}

/// Queries the `concept_graph` table for a co-occurrence edge between two concepts.
/// Returns `Some(map)` where keys are source-file URIs and values are weights,
/// or `None` if no edge exists.
pub(crate) fn get_edge_source_files(
    cfg: &Config,
    concept_a: &str,
    concept_b: &str,
) -> Option<BTreeMap<String, f64>> {
    let (left, right) = if concept_a <= concept_b {
        (concept_a, concept_b)
    } else {
        (concept_b, concept_a)
    };

    db_connection(cfg)
        .query_row(
            "SELECT source_files FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("query concept_graph")
        .and_then(|raw| {
            if raw.trim().is_empty() {
                None
            } else {
                Some(from_str::<BTreeMap<String, f64>>(&raw).expect("parse source_files"))
            }
        })
}
