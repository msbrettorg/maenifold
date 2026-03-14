// T-12.7: RTM NFR-1
#![allow(dead_code)]

use chrono::{Datelike, Duration, TimeZone, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, read_markdown, uri_to_path, write_markdown};

use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use serde_yaml::{Number as YamlNumber, Value as YamlValue};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(10_000);
pub(crate) const WIKILINK_ERROR_HINT: &str = "[[WikiLink]]";

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) struct SessionCleanupEnvGuard {
    previous: Option<String>,
}

impl SessionCleanupEnvGuard {
    pub(crate) fn set_abandonment_minutes(value: &str) -> Self {
        let previous = std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES").ok();
        // SAFETY: tests in this binary hold TEST_MUTEX while mutating process env.
        unsafe { std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", value) }
        Self { previous }
    }
}

impl Drop for SessionCleanupEnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // SAFETY: tests in this binary hold TEST_MUTEX while mutating process env.
            unsafe { std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", value) }
        } else {
            // SAFETY: tests in this binary hold TEST_MUTEX while mutating process env.
            unsafe { std::env::remove_var("MAENIFOLD_SESSION_ABANDON_MINUTES") }
        }
    }
}

pub(crate) fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    fs::create_dir_all(cfg.assets_path.join("workflows")).expect("workflow dir");
    create_test_workflow(&cfg, "wsup-alpha", 3);
    create_test_workflow(&cfg, "wsup-beta", 2);
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn create_test_workflow(cfg: &Config, name: &str, step_count: usize) {
    let workflow = json!({
        "id": name,
        "name": format!("Test Supervisor Workflow {name}"),
        "description": "Test workflow for [[workflow]] supervisor coverage",
        "steps": (1..=step_count)
            .map(|i| json!({
                "id": format!("step-{i}"),
                "name": format!("Step {i}"),
                "description": format!("Test step {i} for [[hierarchical-state-machines]]"),
            }))
            .collect::<Vec<_>>(),
    });
    fs::write(
        cfg.assets_path
            .join("workflows")
            .join(format!("{name}.json")),
        serde_json::to_string_pretty(&workflow).expect("workflow json"),
    )
    .expect("write workflow");
}

pub(crate) fn invoke_workflow(_cfg: &Config, payload: serde_json::Value) -> String {
    maenifold_core::registry::ToolRegistry::new()
        .try_invoke("Workflow", &payload, OutputFormat::Markdown)
        .expect("workflow")
}

pub(crate) fn invoke_sync(_cfg: &Config) -> String {
    maenifold_core::registry::ToolRegistry::new()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync")
}

pub(crate) fn extract_workflow_session_id(result: &str) -> String {
    result
        .lines()
        .find_map(|line| line.strip_prefix("Created session:"))
        .map(str::trim)
        .map(str::to_string)
        .expect("Created session line")
}

pub(crate) fn yaml_string(frontmatter: &Frontmatter, key: &str) -> String {
    frontmatter
        .get(key)
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn yaml_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|part| part.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

pub(crate) fn session_uri(session_type: &str, session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/{session_type}/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn workflow_uri(session_id: &str) -> String {
    session_uri("workflow", session_id)
}

pub(crate) fn sequential_uri(session_id: &str) -> String {
    session_uri("sequential", session_id)
}

pub(crate) fn path_for_uri(cfg: &Config, uri: &str) -> PathBuf {
    uri_to_path(uri, &cfg.memory_path).expect("uri to path")
}

pub(crate) fn read_frontmatter(cfg: &Config, uri: &str) -> Frontmatter {
    read_markdown(uri, cfg)
        .expect("read markdown")
        .0
        .expect("frontmatter")
}

pub(crate) fn read_body(cfg: &Config, uri: &str) -> String {
    read_markdown(uri, cfg).expect("read markdown").1
}

pub(crate) fn create_thinking_session(cfg: &Config, status: &str) -> String {
    let suffix = 10_000 + (SESSION_COUNTER.fetch_add(1, Ordering::Relaxed) % 90_000);
    let session_id = format!("session-{}-{suffix}", Utc::now().timestamp_millis());
    let uri = sequential_uri(&session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Test Thinking Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "thoughtCount".to_string(),
        YamlValue::Number(YamlNumber::from(1)),
    );
    frontmatter.insert(
        "totalThoughts".to_string(),
        YamlValue::Number(YamlNumber::from(3)),
    );
    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Test Thinking Session\n\nThought 1: Working on [[hierarchical-state-machines]] implementation.\n",
        cfg,
    )
    .expect("write thinking session");
    session_id
}

pub(crate) fn update_thinking_session_status(cfg: &Config, session_id: &str, new_status: &str) {
    let uri = sequential_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, cfg).expect("read thinking session");
    let mut frontmatter = frontmatter.expect("thinking frontmatter");
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String(new_status.to_string()),
    );
    write_markdown(&uri, Some(&frontmatter), &content, cfg).expect("update thinking session");
}

pub(crate) fn create_legacy_workflow_session(cfg: &Config) -> String {
    let session_id = format!("workflow-{}", Utc::now().timestamp_millis());
    let uri = workflow_uri(&session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Legacy Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "queue".to_string(),
        YamlValue::Sequence(vec![YamlValue::String("wsup-alpha".to_string())]),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(YamlNumber::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(YamlNumber::from(0)),
    );
    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Workflow Session\n\nLegacy session content with [[backward-compatibility]] test.\n",
        cfg,
    )
    .expect("write legacy workflow session");
    session_id
}

pub(crate) fn write_session_markdown(cfg: &Config, file_uri: &str, status: &str, body: &str) {
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
    write_markdown(file_uri, Some(&frontmatter), body, cfg).expect("write markdown");
}

pub(crate) fn write_raw_markdown(
    cfg: &Config,
    file_uri: &str,
    frontmatter: &Frontmatter,
    body: &str,
) {
    let path = path_for_uri(cfg, file_uri);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    let mut text = String::new();
    if !frontmatter.is_empty() {
        text.push_str("---\n");
        text.push_str(&serde_yaml::to_string(frontmatter).expect("yaml"));
        text.push_str("---\n\n");
    }
    text.push_str(body);
    fs::write(path, text).expect("write raw markdown");
}

pub(crate) fn seed_file_content(
    cfg: &Config,
    file_uri: &str,
    status: Option<&str>,
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
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(file_path) DO UPDATE SET
             title = excluded.title,
             content = excluded.content,
             file_md5 = excluded.file_md5,
             file_size = excluded.file_size,
             last_indexed = excluded.last_indexed,
             status = excluded.status",
        params![file_uri, title, body, format!("md5-{title}"), i64::try_from(body.len()).expect("body length"), last_indexed, status],
    )
    .expect("seed file_content");
}

pub(crate) fn create_session(
    cfg: &Config,
    file_uri: &str,
    status: &str,
    minutes_ago: i64,
    body: &str,
) {
    write_session_markdown(cfg, file_uri, status, body);
    seed_file_content(
        cfg,
        file_uri,
        Some(status),
        body,
        &iso_minutes_ago(minutes_ago),
    );
}

pub(crate) fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - Duration::minutes(minutes_ago)).to_rfc3339()
}

pub(crate) fn read_frontmatter_status(cfg: &Config, file_uri: &str) -> String {
    read_markdown(file_uri, cfg)
        .expect("read markdown")
        .0
        .and_then(|values| {
            values
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_string)
        })
        .expect("status")
}

pub(crate) fn update_last_indexed_and_status(
    cfg: &Config,
    file_uri: &str,
    last_indexed: &str,
    status: Option<&str>,
) {
    Connection::open(&cfg.database_path)
        .expect("open db")
        .execute(
            "UPDATE file_content SET last_indexed = ?2, status = ?3 WHERE file_path = ?1",
            params![file_uri, last_indexed, status],
        )
        .expect("update indexed state");
}
