//! T-2.3: RTM FR-4 — shared RecentActivity test support.

// T-2.3: RTM FR-4 — shared helpers for RecentActivity split tests.
use chrono::Utc;
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    pub _tmp: TempDir,
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

fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

pub fn invoke_tool(tool: &str, payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke(tool, &payload, OutputFormat::Markdown)
        .unwrap_or_else(|_| panic!("invoke {tool}"))
}

pub fn seed_memory_file(cfg: &Config, relative_path: &str, content: &str) {
    let full_path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(full_path.parent().expect("parent")).expect("memory dir");
    fs::write(full_path, content).expect("memory file");
}

pub fn seed_sequential_file(cfg: &Config, session_id: &str, thought_content: &str) {
    let seq_dir = cfg
        .sequential_path
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&seq_dir).expect("sequential dir");
    let content = format!("# {session_id}\n\n{thought_content}\n");
    fs::write(seq_dir.join(format!("{session_id}.md")), content).expect("sequential file");
}

pub fn seed_workflow_file(cfg: &Config, workflow_id: &str, step_content: &str) {
    let wf_dir = cfg
        .thinking_path
        .join("workflow")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&wf_dir).expect("workflow dir");
    let content = format!("# {workflow_id}\n\n{step_content}\n");
    fs::write(wf_dir.join(format!("{workflow_id}.md")), content).expect("workflow file");
}

pub fn seed_chat_file(cfg: &Config, file_name: &str, content: &str) {
    let chat_dir = cfg
        .memory_path
        .join("chat")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&chat_dir).expect("chat dir");
    fs::write(chat_dir.join(file_name), content).expect("chat file");
}

pub fn set_last_indexed(cfg: &Config, file_name_fragment: &str, timestamp_utc: &str) {
    let fragment = file_name_fragment
        .strip_suffix(".md")
        .unwrap_or(file_name_fragment);
    let conn = open_db(cfg);
    let rows = conn
        .execute(
            "UPDATE file_content SET last_indexed = ?1 WHERE file_path LIKE ?2",
            rusqlite::params![timestamp_utc, format!("%{fragment}%")],
        )
        .expect("update last_indexed");
    assert!(
        rows >= 1,
        "expected to update at least one row matching '{fragment}', updated {rows}"
    );
}

pub fn rendered_item_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_start().starts_with("**"))
        .count()
}
