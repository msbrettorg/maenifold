#![allow(dead_code, unused_imports)]

// T-2.3: RTM FR-4
use chrono::Utc;
use maenifold_core::json::OutputFormat;
use maenifold_core::tools::{RecentActivityRecord, ToolContext};
use maenifold_core::{AssetManager, Config, GraphDatabase, MaenifoldError, ToolRegistry};
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_env(initialize_database: bool) -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    if initialize_database {
        GraphDatabase::initialize_database(&cfg).expect("db init");
    }
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn empty_db_env() -> TestEnv {
    isolated_env(false)
}

pub(crate) fn invoke_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &serde_json::json!({}), OutputFormat::Markdown)
        .expect("invoke Sync")
}

pub(crate) fn invoke_recent_activity(payload: serde_json::Value) -> Result<String, MaenifoldError> {
    ToolRegistry::global().try_invoke("RecentActivity", &payload, OutputFormat::Markdown)
}

pub(crate) fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

pub(crate) fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

pub(crate) fn rendered_item_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_start().starts_with("**"))
        .count()
}

pub(crate) fn seed_sequential_file(cfg: &Config, session_id: &str, thought_content: &str) {
    let seq_dir = cfg
        .sequential_path
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&seq_dir).expect("sequential dir");
    let content = format!("# {session_id}\n\n{thought_content}\n");
    fs::write(seq_dir.join(format!("{session_id}.md")), content).expect("sequential file");
}

pub(crate) fn seed_memory_file(cfg: &Config, relative_path: &str, content: &str) {
    let full_path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(full_path.parent().expect("parent")).expect("memory dir");
    fs::write(full_path, content).expect("memory file");
}

pub(crate) fn seed_chat_file(cfg: &Config, file_name: &str, content: &str) {
    let chat_dir = cfg
        .memory_path
        .join("chat")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&chat_dir).expect("chat dir");
    fs::write(chat_dir.join(file_name), content).expect("chat file");
}

pub(crate) fn set_last_indexed(
    cfg: &Config,
    file_name_fragment_without_extension: &str,
    timestamp_utc: &str,
) {
    let fragment = file_name_fragment_without_extension
        .strip_suffix(".md")
        .unwrap_or(file_name_fragment_without_extension);
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

pub(crate) fn sample_record(
    file_path: &str,
    title: &str,
    last_indexed: &str,
    content: &str,
    status: &str,
) -> RecentActivityRecord {
    sample_record_with_summaries(file_path, title, last_indexed, content, status, None, None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_record_with_summaries(
    file_path: &str,
    title: &str,
    last_indexed: &str,
    content: &str,
    status: &str,
    first: Option<&str>,
    last: Option<&str>,
) -> RecentActivityRecord {
    RecentActivityRecord {
        file_path: file_path.to_string(),
        title: title.to_string(),
        last_indexed: last_indexed.to_string(),
        content: content.to_string(),
        status: status.to_string(),
        first: first.map(str::to_string),
        last: last.map(str::to_string),
    }
}

pub(crate) use maenifold_core::tools::{
    collect_recent_activity_records, format_recent_activity_report,
    recent_activity_determine_file_type, recent_activity_extract_session_id,
    recent_activity_extract_snippet,
};
