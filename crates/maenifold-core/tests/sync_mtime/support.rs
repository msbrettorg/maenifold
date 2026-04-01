use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::params;
use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub fn sync_all(env: &TestEnv) -> String {
    search::sync(&env.ctx_markdown(), &json!({})).expect("sync")
}

pub fn create_test_markdown_file(
    env: &TestEnv,
    filename: &str,
    content: &str,
) -> (PathBuf, String) {
    let folder = env.cfg.memory_path.join("tests").join("sync-mtime");
    fs::create_dir_all(&folder).expect("mkdir");
    let file_path = folder.join(filename);
    fs::write(&file_path, content).expect("write fixture");
    let memory_uri = path_to_uri(&file_path, &env.cfg.memory_path).expect("path to uri");
    (file_path, memory_uri)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContentRow {
    pub last_indexed: Option<String>,
    pub file_md5: Option<String>,
    pub file_size: Option<i64>,
}

fn db_connection(cfg: &Config) -> rusqlite::Connection {
    db::open_connection(&cfg.database_path).expect("db open")
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
        .ok()
}

pub fn get_mention_count(cfg: &Config, memory_uri: &str, concept: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![normalize_concept(concept), memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .ok()
}

pub fn tamper_db_delete_mentions(cfg: &Config, memory_uri: &str) {
    db_connection(cfg)
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![memory_uri],
        )
        .expect("delete mentions");
}

pub fn file_mtime(path: &Path) -> SystemTime {
    fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified time")
}

pub fn file_mtime_rfc3339(path: &Path) -> String {
    DateTime::<Utc>::from(file_mtime(path)).to_rfc3339()
}

pub fn set_file_mtime_system_time(path: &Path, mtime: SystemTime) {
    set_file_mtime(path, FileTime::from_system_time(mtime)).expect("set file mtime");
}

#[cfg(unix)]
pub struct PermissionRestore {
    pub path: PathBuf,
    pub mode: u32,
}

#[cfg(unix)]
impl Drop for PermissionRestore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode));
    }
}

pub fn mtime_plus(path: &Path, seconds: u64) -> SystemTime {
    file_mtime(path) + Duration::from_secs(seconds)
}
