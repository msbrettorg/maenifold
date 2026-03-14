// T-12.0: RTM NFR-1
use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search::sync_single_file;
pub use maenifold_core::sync::{
    SKIP_HASH_GUARD_MARKER, SKIP_MTIME_GUARD_MARKER, SYNCED_MENTIONS_MARKER,
};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{from_str, to_string};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[cfg(unix)]
pub struct UnreadableFileGuard {
    path: PathBuf,
    original_mode: u32,
}

#[cfg(unix)]
impl Drop for UnreadableFileGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;

        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.original_mode));
    }
}

#[cfg(windows)]
pub struct UnreadableFileGuard(std::fs::File);

#[derive(Debug, Clone)]
pub struct FileContentRow {
    pub last_indexed: Option<String>,
    pub file_md5: Option<String>,
    pub file_size: Option<i64>,
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

pub fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

pub fn sync_one(cfg: &Config, path: &Path) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, path).expect("sync single file")
}

pub fn create_test_file(cfg: &Config, content: &str) -> (PathBuf, String) {
    let path = cfg.memory_path.join("tests/incremental-mtime/note.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

pub fn get_file_content_row(cfg: &Config, file_uri: &str) -> Option<FileContentRow> {
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

pub fn get_mention_count(cfg: &Config, file_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, file_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query mention_count")
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
            "SELECT COALESCE(source_files, '') FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("query edge")
        .and_then(|raw| {
            if raw.trim().is_empty() {
                None
            } else {
                Some(from_str::<BTreeMap<String, f64>>(&raw).expect("parse source_files"))
            }
        })
}

pub fn tamper_delete_mentions_and_edges_for_file(cfg: &Config, file_uri: &str) {
    let connection = db_connection(cfg);
    connection
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![file_uri],
        )
        .expect("delete mentions");

    let mut statement = connection
        .prepare(
            "SELECT concept_a, concept_b, COALESCE(source_files, '') \
             FROM concept_graph WHERE source_files LIKE ?1",
        )
        .expect("prepare edge tamper query");
    let edges = statement
        .query_map([format!("%{file_uri}%")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .expect("query tamper edges")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect tamper edges");

    for (concept_a, concept_b, raw_files) in edges {
        let mut source_files =
            from_str::<BTreeMap<String, f64>>(&raw_files).unwrap_or_else(|_| BTreeMap::new());
        source_files.remove(file_uri);

        if source_files.is_empty() {
            connection
                .execute(
                    "DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
                    params![concept_a, concept_b],
                )
                .expect("delete emptied edge");
            continue;
        }

        let co_occurrence_count = source_files.values().copied().sum::<f64>();
        connection
            .execute(
                "UPDATE concept_graph SET co_occurrence_count = ?3, source_files = ?4 \
                 WHERE concept_a = ?1 AND concept_b = ?2",
                params![
                    concept_a,
                    concept_b,
                    co_occurrence_count,
                    to_string(&source_files).expect("serialize source_files")
                ],
            )
            .expect("update tampered edge");
    }
}

pub fn parse_rfc3339_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("parse rfc3339")
        .with_timezone(&Utc)
}

pub fn set_file_mtime_utc(path: &Path, instant: DateTime<Utc>) {
    set_file_mtime(
        path,
        FileTime::from_unix_time(instant.timestamp(), instant.timestamp_subsec_nanos()),
    )
    .expect("set mtime");
}

pub fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}

#[cfg(unix)]
pub fn acquire_unreadable_file_guard(path: &Path) -> UnreadableFileGuard {
    use std::os::unix::fs::PermissionsExt;

    let original_mode = fs::metadata(path).expect("metadata").permissions().mode();
    fs::set_permissions(path, fs::Permissions::from_mode(0o200)).expect("drop read permissions");
    UnreadableFileGuard {
        path: path.to_path_buf(),
        original_mode,
    }
}

#[cfg(windows)]
pub fn acquire_unreadable_file_guard(path: &Path) -> UnreadableFileGuard {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(path)
        .expect("lock file");
    UnreadableFileGuard(file)
}

pub fn normalized(concept: &str) -> String {
    normalize_concept(concept)
}
