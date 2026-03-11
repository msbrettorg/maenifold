#![allow(dead_code)]

use chrono::Utc;
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown;
use maenifold_core::memory;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// T-12.6: RTM NFR-1
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    pub fn ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    pub fn with_csharp_defaults() -> Self {
        let keys = [
            "MAENIFOLD_DECAY_FUNCTION",
            "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
            "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
            "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
            "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
        ];
        let previous = keys
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect::<Vec<_>>();

        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", "30");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", "7");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", "14");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", "14");
        }

        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(key);
                }
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
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub fn extract_uri(write_result: &str) -> String {
    write_result
        .lines()
        .find_map(|line| {
            line.find("memory://")
                .map(|index| line[index..].trim().to_string())
        })
        .expect("memory:// URI in write result")
}

pub fn write_memory_file(
    env: &TestEnv,
    title: &str,
    content: &str,
    folder: Option<&str>,
) -> String {
    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory");
    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    extract_uri(&result)
}

pub fn read_memory_output(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx(), &json!({ "identifier": identifier })).expect("read memory")
}

pub fn edit_memory_output(env: &TestEnv, payload: serde_json::Value) -> String {
    memory::edit_memory(&env.ctx(), &payload).expect("edit memory")
}

pub fn move_memory_output(env: &TestEnv, source: &str, destination: &str) -> String {
    memory::move_memory(
        &env.ctx(),
        &json!({
            "source": source,
            "destination": destination,
        }),
    )
    .expect("move memory")
}

pub fn list_memories_output(env: &TestEnv, path: &str) -> String {
    memory::list_memories(&env.ctx(), &json!({ "path": path })).expect("list memories")
}

pub fn write_raw_memory(env: &TestEnv, relative_path: &str, content: &str) -> String {
    let file_path = env.cfg.memory_path.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&file_path, content).expect("write raw memory");
    markdown::path_to_uri(&file_path, &env.cfg.memory_path).expect("path to uri")
}

pub fn raw_path_for_uri(env: &TestEnv, uri: &str) -> std::path::PathBuf {
    markdown::uri_to_path(uri, &env.cfg.memory_path).expect("uri to path")
}

pub fn block_for_file<'a>(output: &'a str, file_stem: &str) -> &'a str {
    let marker = format!("📄 {file_stem}.md");
    let start = output.find(&marker).expect("file block present");
    let rest = &output[start..];
    let end = rest
        .get(marker.len()..)
        .and_then(|tail| tail.find("\n📄 ").map(|index| index + marker.len()))
        .unwrap_or(rest.len());
    &rest[..end]
}

pub fn metadata_value(output: &str, file_stem: &str, key: &str) -> String {
    let block = block_for_file(output, file_stem);
    block
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&format!("{key}: ")).map(str::to_string))
        .unwrap_or_else(|| panic!("missing {key} in block:\n{block}"))
}

pub fn set_last_accessed(env: &TestEnv, uri: &str, last_accessed: &str) {
    let conn = db::open_connection(&env.cfg.database_path).expect("open db");
    conn.execute(
        "INSERT OR REPLACE INTO file_content (file_path, last_indexed, last_accessed) VALUES (?1, ?2, ?3)",
        params![uri, Utc::now().to_rfc3339(), last_accessed],
    )
    .expect("insert last_accessed");
}
