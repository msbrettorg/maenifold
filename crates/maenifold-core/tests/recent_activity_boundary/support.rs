#![allow(dead_code, unused_imports)]

use chrono::{Duration, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-2.3: RTM FR-4 - share RecentActivity boundary fixtures while splitting coverage by theme.
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

pub fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

pub fn timestamp(hours_ago: i64) -> String {
    (Utc::now() - Duration::hours(hours_ago))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn timestamp_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[allow(clippy::too_many_arguments)]
pub fn seed_activity(
    conn: &Connection,
    file_path: &str,
    title: &str,
    content: &str,
    last_indexed: &str,
    status: &str,
) {
    conn.execute(
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file_path,
            title,
            content,
            format!("md5-{title}"),
            i64::try_from(content.len()).expect("file size"),
            last_indexed,
            status
        ],
    )
    .expect("seed activity");
}

pub fn seed_mixed_activity(conn: &Connection) {
    seed_activity(
        conn,
        "memory://thinking/sequential/2026/03/session-recent",
        "Sequential Session",
        "## Thought 1\nFirst sequential thought\n## Thought 2\nSecond sequential thought",
        &timestamp(1),
        "active",
    );
    seed_activity(
        conn,
        "memory://thinking/workflow/2026/03/workflow-middle",
        "Workflow Session",
        "## Step 1\nCollect evidence\n## Step 2\nSummarize findings",
        &timestamp(2),
        "completed",
    );
    seed_activity(
        conn,
        "memory://projects/security/knowledge-note",
        "Knowledge Note",
        "# Knowledge Note\n\n## Intro\nDocumented memory content\n\n## Outro\nClosing memory section",
        &timestamp(3),
        "active",
    );
    seed_activity(
        conn,
        "memory://chat/2026/03/assumption-log",
        "Assumption Log",
        "# Assumption Log\n\n## Message\nTracked assumption content",
        &timestamp(4),
        "active",
    );
}

pub fn invoke_recent_activity(payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke("RecentActivity", &payload, OutputFormat::Markdown)
        .expect("invoke RecentActivity")
}

pub fn rendered_item_count(output: &str) -> usize {
    output.lines().filter(|line| line.starts_with("**")).count()
}
