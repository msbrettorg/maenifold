#![allow(dead_code, unused_imports)]

use chrono::{Duration, Timelike, Utc};
use maenifold_core::db;
use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
use maenifold_core::sessions;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use rusqlite::{Connection, params};
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

// Re-export the process-wide mutex so tests across all modules are serialised on the same lock.
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct SessionCleanupEnvGuard {
    previous: Option<String>,
}

impl SessionCleanupEnvGuard {
    pub fn set_abandonment_minutes(value: &str) -> Self {
        let previous = std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES").ok();
        // SAFETY: tests in this module hold TEST_MUTEX for their full duration,
        // so this process-wide environment mutation is serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", value);
        }
        Self { previous }
    }
}

impl Drop for SessionCleanupEnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // SAFETY: tests in this module hold TEST_MUTEX for their full duration,
            // so this process-wide environment mutation is serialized.
            unsafe {
                std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", value);
            }
        } else {
            // SAFETY: tests in this module hold TEST_MUTEX for their full duration,
            // so this process-wide environment mutation is serialized.
            unsafe {
                std::env::remove_var("MAENIFOLD_SESSION_ABANDON_MINUTES");
            }
        }
    }
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - Duration::minutes(minutes_ago)).to_rfc3339()
}

/// Sleep until the next whole-second boundary so that `Utc::now()` calls
/// made immediately before and after DB seeding share the same truncated
/// second.  Mirrors `workflow_supervisor/sync_support.rs`.
pub fn align_to_second_boundary() {
    let nanos = Utc::now().nanosecond();
    if nanos > 900_000_000 {
        std::thread::sleep(std::time::Duration::from_millis(150));
        return;
    }
    let wait = 1_000_000_000u32 - nanos + 20_000_000;
    std::thread::sleep(std::time::Duration::from_nanos(u64::from(wait)));
}

pub fn write_session_markdown(cfg: &Config, file_uri: &str, status: &str, body: &str) {
    write_session_markdown_with_pipeline(cfg, file_uri, status, body, None)
}

pub fn write_session_markdown_with_pipeline(
    cfg: &Config,
    file_uri: &str,
    status: &str,
    body: &str,
    pipeline_id: Option<&str>,
) {
    let mut frontmatter = Frontmatter::new();
    let title = file_uri
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .expect("title segment");
    let session_type = if file_uri.contains("/workflow/") {
        "workflow"
    } else {
        "sequential"
    };
    frontmatter.insert("title".to_string(), YamlValue::String(title.to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String(session_type.to_string()),
    );
    if let Some(pid) = pipeline_id {
        frontmatter.insert("pipelineId".to_string(), YamlValue::String(pid.to_string()));
    }
    write_markdown(file_uri, Some(&frontmatter), body, cfg).expect("write markdown");
}

pub fn seed_file_content(
    cfg: &Config,
    file_uri: &str,
    status: &str,
    body: &str,
    last_indexed: &str,
) {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    let title = file_uri
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .expect("title segment");
    conn.execute(
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file_uri,
            title,
            body,
            format!("md5-{title}"),
            i64::try_from(body.len()).expect("body length"),
            last_indexed,
            status,
        ],
    )
    .expect("seed file_content");
}

pub fn create_session(cfg: &Config, file_uri: &str, status: &str, minutes_ago: i64, body: &str) {
    write_session_markdown(cfg, file_uri, status, body);
    seed_file_content(cfg, file_uri, status, body, &iso_minutes_ago(minutes_ago));
}

#[allow(clippy::too_many_arguments)]
pub fn create_pipeline_session(
    cfg: &Config,
    file_uri: &str,
    status: &str,
    minutes_ago: i64,
    body: &str,
    pipeline_id: &str,
) {
    write_session_markdown_with_pipeline(cfg, file_uri, status, body, Some(pipeline_id));
    seed_file_content(cfg, file_uri, status, body, &iso_minutes_ago(minutes_ago));
}

pub fn cleanup_abandoned_sessions(cfg: &Config) -> usize {
    sessions::cleanup_abandoned_sessions(cfg).expect("cleanup")
}

pub fn read_frontmatter_status(cfg: &Config, file_uri: &str) -> String {
    let (frontmatter, _, _) = read_markdown(file_uri, cfg).expect("read markdown");
    frontmatter
        .and_then(|values| {
            values
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_string)
        })
        .expect("status")
}

pub fn read_body(cfg: &Config, file_uri: &str) -> String {
    let (_, body, _) = read_markdown(file_uri, cfg).expect("read markdown");
    body
}

pub fn read_db_status(cfg: &Config, file_uri: &str) -> String {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row(
        "SELECT status FROM file_content WHERE file_path = ?1",
        params![file_uri],
        |row| row.get::<_, String>(0),
    )
    .expect("db status")
}

pub fn read_db_content(cfg: &Config, file_uri: &str) -> String {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row(
        "SELECT content FROM file_content WHERE file_path = ?1",
        params![file_uri],
        |row| row.get::<_, String>(0),
    )
    .expect("db content")
}

pub fn file_content_row_count(cfg: &Config) -> i64 {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row("SELECT COUNT(*) FROM file_content", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("row count")
}
