use chrono::{Duration, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
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

fn timestamp(hours_ago: i64) -> String {
    (Utc::now() - Duration::hours(hours_ago))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn timestamp_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn seed_activity(
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

fn seed_mixed_activity(conn: &Connection) {
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

fn invoke_recent_activity(payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke("RecentActivity", &payload, OutputFormat::Markdown)
        .expect("invoke RecentActivity")
}

fn rendered_item_count(output: &str) -> usize {
    output.lines().filter(|line| line.starts_with("**")).count()
}

// T-2.5a: RTM FR-4 (TC-4.1 / RTM-020)
#[test]
fn recent_activity_default_query_orders_by_last_indexed_desc() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert!(
        output.starts_with("# Recent Activity"),
        "unexpected output: {output}"
    );
    let sequential_idx = output.find("**session-recent**").expect("sequential item");
    let workflow_idx = output.find("**workflow-middle**").expect("workflow item");
    let memory_idx = output.find("**knowledge-note**").expect("memory item");
    let assumptions_idx = output.find("**assumption-log**").expect("assumption item");
    assert!(
        sequential_idx < workflow_idx && workflow_idx < memory_idx && memory_idx < assumptions_idx,
        "items should be ordered by last_indexed desc: {output}"
    );
}

// T-2.5a: RTM FR-4 (TC-4.2 / RTM-021 / RTM-022 / RTM-023)
#[test]
fn recent_activity_filter_variants_select_expected_path_families() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let thinking = invoke_recent_activity(json!({ "limit": 10, "filter": "thinking" }));
    assert!(thinking.contains("**session-recent** (sequential)"));
    assert!(thinking.contains("**workflow-middle** (workflow)"));
    assert!(!thinking.contains("**knowledge-note**"));
    assert!(!thinking.contains("**assumption-log**"));

    let memory = invoke_recent_activity(json!({ "limit": 10, "filter": "memory" }));
    assert!(memory.contains("**knowledge-note** (memory)"));
    assert!(!memory.contains("**session-recent**"));
    assert!(!memory.contains("**workflow-middle**"));
    assert!(!memory.contains("**assumption-log**"));

    let assumptions = invoke_recent_activity(json!({ "limit": 10, "filter": "assumptions" }));
    assert!(assumptions.contains("**assumption-log** (memory)"));
    assert!(assumptions.contains("Title: Assumption Log"));
    assert!(!assumptions.contains("**session-recent**"));
    assert!(!assumptions.contains("**workflow-middle**"));
    assert!(!assumptions.contains("**knowledge-note**"));
}

// T-2.5a: RTM FR-4 (TC-4.3 / RTM-024)
#[test]
fn recent_activity_timespan_restricts_results_to_requested_window() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-fresh",
        "Fresh Session",
        "## Thought 1\nFresh activity",
        &timestamp(2),
        "active",
    );
    seed_activity(
        &conn,
        "memory://projects/archive/session-stale",
        "Stale Memory",
        "# Stale\n\n## Intro\nOut of window",
        &timestamp_days_ago(3),
        "active",
    );

    let output = invoke_recent_activity(json!({
        "limit": 10,
        "timespan": "24.00:00:00"
    }));

    assert!(
        output.contains("**session-fresh**"),
        "expected fresh row: {output}"
    );
    assert!(
        !output.contains("**session-stale**"),
        "stale row should be excluded by timespan: {output}"
    );
}

// T-2.5a: RTM FR-4 (TC-4.4 / RTM-025 / RTM-030)
#[test]
fn recent_activity_include_content_includes_h2_heading_and_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-content",
        "Content Session",
        "## Thought 1\nFirst content block\n## Thought 2\nSecond content block",
        &timestamp(1),
        "active",
    );

    let output = invoke_recent_activity(json!({
        "limit": 10,
        "includeContent": true
    }));

    assert!(
        output.contains("First H2: Thought 1"),
        "missing first H2: {output}"
    );
    assert!(
        output.contains("First H2 Content: First content block"),
        "missing first H2 content: {output}"
    );
    assert!(
        output.contains("Last H2: Thought 2"),
        "missing last H2: {output}"
    );
    assert!(
        output.contains("Last H2 Content: Second content block"),
        "missing last H2 content: {output}"
    );
}

// T-2.5a: RTM FR-4 (TC-4.5 / RTM-026)
#[test]
fn recent_activity_limit_caps_number_of_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let output = invoke_recent_activity(json!({ "limit": 2 }));

    assert_eq!(
        rendered_item_count(&output),
        2,
        "unexpected output: {output}"
    );
    assert!(output.contains("**session-recent**"));
    assert!(output.contains("**workflow-middle**"));
    assert!(!output.contains("**knowledge-note**"));
    assert!(!output.contains("**assumption-log**"));
}

// T-2.5a: RTM FR-4 (TC-4.6 / RTM-027)
#[test]
fn recent_activity_returns_no_activity_message_when_query_matches_nothing() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert_eq!(output.trim(), "No recent activity found");
}

// T-2.5a: RTM FR-4 (TC-4.7 / RTM-028)
#[test]
fn recent_activity_learn_true_returns_help_documentation() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_recent_activity(json!({ "learn": true }));

    assert!(
        output.contains("RecentActivity"),
        "missing tool name: {output}"
    );
    assert!(
        output.contains("timespan"),
        "missing parameter docs: {output}"
    );
    assert!(
        output.contains("Include full section content") || output.contains("includeContent"),
        "missing content parameter docs: {output}"
    );
    assert!(
        !output.starts_with("# Recent Activity"),
        "learn=true should return help, not live activity output: {output}"
    );
}

// T-2.5a: RTM FR-4 (TC-4.8 / RTM-029)
#[test]
fn recent_activity_classifies_sequential_workflow_and_memory_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-classified",
        "Sequential Classified",
        "## Thought 1\nClassified sequential",
        &timestamp(1),
        "active",
    );
    seed_activity(
        &conn,
        "memory://thinking/workflow/2026/03/workflow-classified",
        "Workflow Classified",
        "## Step 1\nClassified workflow",
        &timestamp(2),
        "completed",
    );
    seed_activity(
        &conn,
        "memory://projects/reference/memory-classified",
        "Memory Classified",
        "# Memory Classified\n\n## Intro\nClassified memory",
        &timestamp(3),
        "active",
    );

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert!(output.contains("**session-classified** (sequential)"));
    assert!(output.contains("Thoughts: 1"));
    assert!(output.contains("**workflow-classified** (workflow)"));
    assert!(output.contains("Steps: 1"));
    assert!(output.contains("**memory-classified** (memory)"));
    assert!(output.contains("Title: Memory Classified"));
}
