#![allow(dead_code, unused_imports)]

// T-5.7: FR archived — see CHANGELOG.md § FR-14 (RTM-080). Incremental sync DB-coherence
// test support. Shared by both lifecycle and guards test files.
use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::from_str;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Guards Config::override_root() process-global state, not DB or filesystem contention.
// Each test has its own TempDir and DB path, but override_root() mutates a process-global
// config object. Without this mutex, concurrent test threads would clobber each other's
// config even though their TempDirs are isolated.
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

const TEST_SUBFOLDER: &str = "tests/incremental-unify";

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
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

pub fn sync_path(cfg: &Config, path: &Path) -> String {
    let context = ToolContext::new(cfg.clone(), maenifold_core::json::OutputFormat::Markdown);
    sync_single_file(&context, path).expect("sync single file")
}

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub fn create_test_file(
    cfg: &Config,
    content: &str,
    name_prefix: Option<&str>,
) -> (PathBuf, String) {
    let test_folder = cfg.memory_path.join(TEST_SUBFOLDER);
    fs::create_dir_all(&test_folder).expect("mkdir");

    let unique_suffix = uuid::Uuid::new_v4().simple().to_string();
    let prefix = name_prefix.unwrap_or("note");
    let file_path = test_folder.join(format!("{prefix}-{unique_suffix}.md"));
    fs::write(&file_path, content).expect("write fixture");

    let memory_uri = path_to_uri(&file_path, &cfg.memory_path).expect("file uri");
    (file_path, memory_uri)
}

#[derive(Debug)]
pub struct FileContentRow {
    pub last_indexed: Option<String>,
    pub file_md5: Option<String>,
    pub file_size: Option<i64>,
}

pub fn get_file_content_row(cfg: &Config, memory_uri: &str) -> Option<FileContentRow> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = ?1",
            [memory_uri],
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

pub fn get_mention_count(cfg: &Config, memory_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query concept_mentions")
}

pub fn get_edge_source_files(
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

pub fn tamper_delete_mentions_and_edges_for_file(cfg: &Config, memory_uri: &str) {
    let connection = db_connection(cfg);
    connection
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![memory_uri],
        )
        .expect("delete concept_mentions");

    let mut statement = connection
        .prepare("SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE ?1")
        .expect("prepare edge query");
    let edges = statement
        .query_map([format!("%{memory_uri}%")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .expect("query edges")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect edges");

    for (concept_a, concept_b, files) in edges {
        let mut file_dict =
            from_str::<BTreeMap<String, f64>>(&files).unwrap_or_else(|_| BTreeMap::new());
        file_dict.remove(memory_uri);

        if file_dict.is_empty() {
            connection
                .execute(
                    "DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
                    params![concept_a, concept_b],
                )
                .expect("delete empty edge");
            continue;
        }

        connection
            .execute(
                "UPDATE concept_graph SET co_occurrence_count = ?3, source_files = ?4 WHERE concept_a = ?1 AND concept_b = ?2",
                params![
                    concept_a,
                    concept_b,
                    file_dict.len() as i64,
                    serde_json::to_string(&file_dict).expect("serialize source_files"),
                ],
            )
            .expect("update edge");
    }
}

pub fn parse_rfc3339_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("parse timestamp")
        .with_timezone(&Utc)
}

pub fn set_file_timestamp(path: &Path, timestamp: DateTime<Utc>) {
    let file_time =
        FileTime::from_unix_time(timestamp.timestamp(), timestamp.timestamp_subsec_nanos());
    set_file_mtime(path, file_time).expect("set file mtime");
}

pub fn normalized_concept(value: &str) -> String {
    normalize_concept(value)
}

// Read filesystem mtime as RFC3339 for exact-value assertions
pub fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}
