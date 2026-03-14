#![allow(dead_code)]

// T-0.3: RTM NFR-4
use chrono::{DateTime, Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory::{self, compute_decay_weight};
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
pub const TEST_FOLDER: &str = "graph-decay-weighting-tests";

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    pub fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
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

pub fn memory_uri_from_relative(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

pub fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

pub fn create_test_file_with_age(
    cfg: &Config,
    relative_path: &str,
    title: &str,
    content: &str,
    days_ago: i64,
) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    let parent = file_path.parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create test dirs");

    let created_iso = rfc3339_days_ago(days_ago);
    let full_content = format!(
        "---\n\
         title: {title}\n\
         type: memory\n\
         status: saved\n\
         created: {created_iso}\n\
         modified: {created_iso}\n\
         ---\n\n\
         # {title}\n\n\
         {content}\n"
    );
    fs::write(&file_path, full_content).expect("write memory file");

    memory_uri_from_relative(relative_path)
}

pub fn sync_graph(env: &TestEnv) {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
}

pub fn search_memories_json(env: &TestEnv, query: &str, mode: &str) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": query,
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str(&raw).expect("search memories json")
}

pub fn build_context_json(env: &TestEnv, concept_name: &str) -> Value {
    let raw = search::build_context(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "depth": 1,
            "maxEntities": 20,
            "includeContent": false,
        }),
    )
    .expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

pub fn find_similar_concepts_markdown(env: &TestEnv, concept_name: &str) -> String {
    search::find_similar_concepts(
        &env.ctx_markdown(),
        &json!({
            "conceptName": concept_name,
            "maxResults": 20,
        }),
    )
    .expect("find similar concepts")
}

pub fn find_similar_concepts_json(env: &TestEnv, concept_name: &str) -> Value {
    let raw = search::find_similar_concepts(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "maxResults": 20,
        }),
    )
    .expect("find similar concepts");
    serde_json::from_str(&raw).expect("find similar concepts json")
}

pub fn concept_index(response: &Value, concept_name: &str) -> usize {
    response["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .position(|entry| entry["name"].as_str() == Some(concept_name))
        .unwrap_or_else(|| panic!("missing concept for {concept_name:?}: {response}"))
}

pub fn read_memory_markdown(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
}

pub fn find_result<'a>(response: &'a Value, title: &str) -> &'a Value {
    response["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .find(|entry| entry["title"].as_str() == Some(title))
        .unwrap_or_else(|| panic!("missing result for {title:?}: {response}"))
}

pub fn title_index(response: &Value, title: &str) -> usize {
    response["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .position(|entry| entry["title"].as_str() == Some(title))
        .unwrap_or_else(|| panic!("missing result for {title:?}: {response}"))
}

pub fn get_last_accessed(cfg: &Config, uri: &str) -> Option<DateTime<Utc>> {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let raw = connection
        .query_row(
            "SELECT last_accessed FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("query last_accessed")?;

    Some(
        DateTime::parse_from_rfc3339(&raw)
            .expect("valid rfc3339 timestamp")
            .with_timezone(&Utc),
    )
}

pub fn decay_weight_for_uri(cfg: &Config, uri: &str) -> f64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let (last_accessed, status) = connection
        .query_row(
            "SELECT last_accessed, status FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .expect("query decay metadata");

    compute_decay_weight(uri, last_accessed.as_deref(), cfg, status.as_deref())
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}
