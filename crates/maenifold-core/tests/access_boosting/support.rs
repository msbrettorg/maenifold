// T-12.0: RTM NFR-1
#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::TempDir;

pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(crate) const TEST_FOLDER: &str = "access-boosting-tests";
pub(crate) const READ_SLEEP_MS: u64 = 1_100;

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl TestEnv {
    pub(crate) fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    pub(crate) fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn create_and_sync_test_file(
    env: &TestEnv,
    title: &str,
    content: &str,
    folder: &str,
) -> String {
    let result = memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory");
    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected write result: {result}"
    );

    let uri = extract_uri(&result);
    sync_graph(env);
    uri
}

pub(crate) fn read_memory_output(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
}

pub(crate) fn search_memories_json(
    env: &TestEnv,
    query: &str,
    mode: &str,
    folder: Option<&str>,
) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": query,
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": folder,
            "tags": Value::Null,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str(&raw).expect("search output json")
}

pub(crate) fn build_context_json(
    env: &TestEnv,
    concept_name: &str,
    depth: usize,
    max_entities: usize,
    include_content: bool,
) -> Value {
    let raw = search::build_context(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "depth": depth,
            "maxEntities": max_entities,
            "includeContent": include_content,
        }),
    )
    .expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

pub(crate) fn get_last_accessed(cfg: &Config, uri: &str) -> Option<DateTime<Utc>> {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    let raw = conn
        .query_row(
            "SELECT last_accessed FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .expect("query last_accessed")
        .flatten()?;

    Some(
        DateTime::parse_from_rfc3339(&raw)
            .expect("valid rfc3339 timestamp")
            .with_timezone(&Utc),
    )
}

pub(crate) fn set_last_accessed(cfg: &Config, uri: &str, last_accessed: &str) {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    conn.execute(
        "UPDATE file_content SET last_accessed = ?1 WHERE file_path = ?2",
        params![last_accessed, uri],
    )
    .expect("update last_accessed");
}

pub(crate) fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

pub(crate) fn assert_search_contains_title(search_result: &Value, title: &str) {
    let results = search_result["data"]["results"]
        .as_array()
        .expect("results array");
    assert!(
        results
            .iter()
            .any(|entry| entry["title"].as_str() == Some(title)),
        "expected search results to contain {title:?}: {search_result}"
    );
}

fn extract_uri(write_result: &str) -> String {
    write_result
        .lines()
        .find_map(|line| {
            line.find("memory://")
                .map(|index| line[index..].trim().to_string())
        })
        .expect("memory:// URI in write result")
}

fn sync_graph(env: &TestEnv) {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
}
