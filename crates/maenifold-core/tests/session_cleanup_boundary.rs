use chrono::{Duration, Utc};
use maenifold_core::db;
use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
use maenifold_core::sessions;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_yaml::Value as YamlValue;
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

struct SessionCleanupEnvGuard {
    previous: Option<String>,
}

impl SessionCleanupEnvGuard {
    fn set_abandonment_minutes(value: &str) -> Self {
        let previous = std::env::var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES").ok();
        // SAFETY: tests in this file hold TEST_MUTEX for their full duration,
        // so this process-wide environment mutation is serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES", value);
        }
        Self { previous }
    }
}

impl Drop for SessionCleanupEnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // SAFETY: tests in this file hold TEST_MUTEX for their full duration,
            // so this process-wide environment mutation is serialized.
            unsafe {
                std::env::set_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES", value);
            }
        } else {
            // SAFETY: tests in this file hold TEST_MUTEX for their full duration,
            // so this process-wide environment mutation is serialized.
            unsafe {
                std::env::remove_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES");
            }
        }
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

fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - Duration::minutes(minutes_ago)).to_rfc3339()
}

fn write_session_markdown(cfg: &Config, file_uri: &str, status: &str, body: &str) {
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

fn seed_file_content(cfg: &Config, file_uri: &str, status: &str, body: &str, last_indexed: &str) {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    let title = file_uri
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .expect("title segment");
    conn.execute(
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file_uri,
            title,
            body,
            format!("md5-{title}"),
            i64::try_from(body.len()).expect("body length"),
            last_indexed,
            status,
        ],
    )
    .expect("seed file_content");
}

fn create_session(cfg: &Config, file_uri: &str, status: &str, minutes_ago: i64, body: &str) {
    write_session_markdown(cfg, file_uri, status, body);
    seed_file_content(cfg, file_uri, status, body, &iso_minutes_ago(minutes_ago));
}

fn read_frontmatter_status(cfg: &Config, file_uri: &str) -> String {
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

fn read_body(cfg: &Config, file_uri: &str) -> String {
    let (_, body, _) = read_markdown(file_uri, cfg).expect("read markdown");
    body
}

fn read_db_status(cfg: &Config, file_uri: &str) -> String {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row(
        "SELECT status FROM file_content WHERE file_path = ?1",
        params![file_uri],
        |row| row.get::<_, String>(0),
    )
    .expect("db status")
}

fn read_db_content(cfg: &Config, file_uri: &str) -> String {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row(
        "SELECT content FROM file_content WHERE file_path = ?1",
        params![file_uri],
        |row| row.get::<_, String>(0),
    )
    .expect("db content")
}

fn file_content_row_count(cfg: &Config) -> i64 {
    let conn = Connection::open(&cfg.database_path).expect("open db");
    conn.query_row("SELECT COUNT(*) FROM file_content", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("row count")
}

// T-4.5b: RTM FR-4.2
#[test]
fn active_session_within_default_ttl_is_not_marked_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-within-ttl";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        30,
        "# Session\n\n## Thought 1\nStill active [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_db_status(&env.cfg, file_uri), "active");
    let body = read_body(&env.cfg, file_uri);
    assert!(
        !body.contains("## Session Abandoned"),
        "unexpected body: {body}"
    );
}

// T-4.5b: RTM FR-4.2
#[test]
fn active_session_beyond_default_ttl_is_marked_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-beyond-ttl";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        180,
        "# Session\n\n## Thought 1\nOld activity [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    assert_eq!(read_db_status(&env.cfg, file_uri), "abandoned");
    let body = read_body(&env.cfg, file_uri);
    assert!(
        body.contains("## Session Abandoned"),
        "missing notice: {body}"
    );
    let db_content = read_db_content(&env.cfg, file_uri);
    assert!(
        db_content.contains("## Session Abandoned"),
        "db content should be updated too: {db_content}"
    );
}

// T-4.5b: RTM FR-4.2
#[test]
fn completed_and_cancelled_sessions_are_not_touched_regardless_of_age() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let completed_uri = "memory://thinking/sequential/2026/03/09/session-completed";
    let cancelled_uri = "memory://thinking/workflow/2026/03/09/workflow-cancelled";
    create_session(
        &env.cfg,
        completed_uri,
        "completed",
        600,
        "# Session\n\n## Thought 1\nFinished [[Session Cleanup]]",
    );
    create_session(
        &env.cfg,
        cancelled_uri,
        "cancelled",
        600,
        "# Workflow Session\n\n## Step 1\nCancelled [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    for (file_uri, expected_status) in [(completed_uri, "completed"), (cancelled_uri, "cancelled")]
    {
        assert_eq!(read_frontmatter_status(&env.cfg, file_uri), expected_status);
        assert_eq!(read_db_status(&env.cfg, file_uri), expected_status);
        let body = read_body(&env.cfg, file_uri);
        assert!(
            !body.contains("## Session Abandoned"),
            "unexpected body: {body}"
        );
    }
}

// T-4.5b: RTM FR-4.2
#[test]
fn multiple_abandoned_sessions_are_detected_in_one_sweep() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let old_sequential = "memory://thinking/sequential/2026/03/09/session-old-1";
    let old_workflow = "memory://thinking/workflow/2026/03/09/workflow-old-1";
    let recent_active = "memory://thinking/sequential/2026/03/09/session-recent";

    create_session(
        &env.cfg,
        old_sequential,
        "active",
        240,
        "# Session\n\n## Thought 1\nOld [[Session Cleanup]] session",
    );
    create_session(
        &env.cfg,
        old_workflow,
        "active",
        360,
        "# Workflow Session\n\n## Step 1\nOld [[Session Cleanup]] workflow",
    );
    create_session(
        &env.cfg,
        recent_active,
        "active",
        15,
        "# Session\n\n## Thought 1\nRecent [[Session Cleanup]] session",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 2);
    assert_eq!(
        read_frontmatter_status(&env.cfg, old_sequential),
        "abandoned"
    );
    assert_eq!(read_frontmatter_status(&env.cfg, old_workflow), "abandoned");
    assert_eq!(read_frontmatter_status(&env.cfg, recent_active), "active");
}

// T-4.5b: RTM FR-4.2
#[test]
fn empty_database_returns_zero_abandoned_sessions() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(file_content_row_count(&env.cfg), 0);
}

// T-4.5b: RTM FR-4.2
#[test]
fn custom_ttl_from_env_var_is_respected() {
    let _guard = TEST_MUTEX.lock();
    let _env_guard = SessionCleanupEnvGuard::set_abandonment_minutes("5");
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-custom-ttl";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        10,
        "# Session\n\n## Thought 1\nShort TTL [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    let body = read_body(&env.cfg, file_uri);
    assert!(
        body.contains("## Session Abandoned"),
        "missing notice: {body}"
    );
}
