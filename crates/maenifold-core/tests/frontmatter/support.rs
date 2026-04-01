#![allow(dead_code, unused_imports)]

use chrono::{Duration as ChronoDuration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{self, Frontmatter, read_markdown};
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use rusqlite::{Connection, params};
use serde_json::Value;
use serde_yaml::Value as YamlValue;
use std::fs;
use tempfile::TempDir;

// T-12.6: RTM NFR-1 — Shared support keeps split frontmatter regression coverage behaviorally identical.
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

pub fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

pub fn json_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Json)
}

pub fn yaml_string(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(text) => Some(text.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        other => serde_yaml::to_string(other)
            .ok()
            .map(|text| text.trim().to_string()),
    })
}

pub fn write_raw_markdown(cfg: &Config, file_uri: &str, raw_text: &str) {
    let path = markdown::uri_to_path(file_uri, &cfg.memory_path).expect("uri to path");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, raw_text).expect("write raw markdown");
}

pub fn read_raw_markdown(cfg: &Config, file_uri: &str) -> String {
    let path = markdown::uri_to_path(file_uri, &cfg.memory_path).expect("uri to path");
    fs::read_to_string(path).expect("read raw markdown")
}

pub fn extract_uri_from_json(output: &str) -> String {
    let value: Value = serde_json::from_str(output).expect("tool json");
    assert_eq!(value["success"], true, "tool call failed: {output}");
    value["data"]["uri"].as_str().expect("uri").to_string()
}

pub fn extract_memory_uris(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|token| token.starts_with("memory://"))
        .map(|token| token.trim_matches(|ch: char| ch == ',' || ch == ';'))
        .map(str::to_string)
        .collect()
}

pub fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - ChronoDuration::minutes(minutes_ago)).to_rfc3339()
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
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file_uri,
            title,
            body,
            format!("md5-{title}"),
            i64::try_from(body.len()).expect("body len"),
            last_indexed,
            status,
        ],
    )
    .expect("seed file_content");
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
