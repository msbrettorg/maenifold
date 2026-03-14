#![allow(dead_code)]

// T-12.7: RTM NFR-1
use chrono::{Duration, Utc};
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
pub const TEST_FOLDER: &str = "decay-benchmark-tests";

// T-12.7: RTM NFR-1 — all decay-related env vars that influence test behavior
const DECAY_ENV_KEYS: [&str; 5] = [
    "MAENIFOLD_DECAY_FUNCTION",
    "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
    "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
    "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
    "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
];

/// RAII guard that captures current decay-related env vars on creation
/// and restores them on drop, ensuring test hermeticity.
pub struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    /// Pin all decay env vars to known defaults so tests are hermetic
    /// regardless of the developer's shell environment.
    ///
    /// Pinned values match production defaults:
    ///   - decay function: power-law (NFR-1 baseline formula)
    ///   - half-life: 30 days (exponential fallback parameter)
    ///   - sequential grace: 7 days (episodic memory tier)
    ///   - workflow grace: 14 days (workflow memory tier)
    ///   - default grace: 28 days (semantic memory tier)
    pub fn pin_defaults() -> Self {
        let previous = DECAY_ENV_KEYS
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect();

        // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
        // so process-wide environment mutations are serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", "30");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", "7");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", "14");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", "28");
        }

        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(v) => {
                    // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::set_var(key, v) };
                }
                None => {
                    // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::remove_var(key) };
                }
            }
        }
    }
}

/// Compute the expected power-law decay weight for a given age.
///
/// Formula: `weight = max(1, age_days - grace_days)^(-0.5)`, returns 1.0
/// during grace period. Mirrors `scoring.rs` logic for test verification.
pub fn expected_power_law_weight(age_days: i64, grace_days: i64) -> f64 {
    if age_days <= grace_days {
        return 1.0;
    }
    let effective_age = (age_days - grace_days).max(1) as f64;
    effective_age.powf(-0.5)
}

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
    let escaped_title = title.replace('"', "\\\"");
    let full_content = format!(
        "---\n\
         title: \"{escaped_title}\"\n\
         type: \"memory\"\n\
         status: \"saved\"\n\
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

pub fn read_memory_markdown(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
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

pub fn find_document_rank(search_result: &Value, title_fragment: &str) -> Option<usize> {
    search_result["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .position(|entry| {
            entry["title"]
                .as_str()
                .map(|title| title.contains(title_fragment))
                .unwrap_or(false)
        })
}

// T-12.7: RTM NFR-1 — find a result object by title fragment for field assertions
pub fn find_result<'a>(search_result: &'a Value, title_fragment: &str) -> Option<&'a Value> {
    search_result["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .find(|entry| {
            entry["title"]
                .as_str()
                .map(|title| title.contains(title_fragment))
                .unwrap_or(false)
        })
}

pub fn format_rank(rank: Option<usize>) -> String {
    rank.map(|value| format!("#{}", value + 1))
        .unwrap_or_else(|| "NOT FOUND".to_string())
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}
