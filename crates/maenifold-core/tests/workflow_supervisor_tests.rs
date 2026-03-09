use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{
    Frontmatter, path_to_uri, read_markdown, uri_to_path, write_markdown,
};
use maenifold_core::search;
use maenifold_core::sessions;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use serde_yaml::{Number as YamlNumber, Value as YamlValue};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(10_000);

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
    fs::create_dir_all(cfg.assets_path.join("workflows")).expect("workflow dir");
    create_test_workflow(&cfg, "wsup-alpha", 3);
    create_test_workflow(&cfg, "wsup-beta", 2);
    TestEnv { _tmp: tmp, cfg }
}

fn tool_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

fn create_test_workflow(cfg: &Config, name: &str, step_count: usize) {
    let workflow = json!({
        "id": name,
        "name": format!("Test Supervisor Workflow {name}"),
        "description": "Test workflow for [[workflow]] supervisor coverage",
        "steps": (1..=step_count)
            .map(|i| json!({
                "id": format!("step-{i}"),
                "name": format!("Step {i}"),
                "description": format!("Test step {i} for [[hierarchical-state-machines]]"),
            }))
            .collect::<Vec<_>>(),
    });
    fs::write(
        cfg.assets_path
            .join("workflows")
            .join(format!("{name}.json")),
        serde_json::to_string_pretty(&workflow).expect("workflow json"),
    )
    .expect("write workflow");
}

fn invoke_workflow(cfg: &Config, payload: serde_json::Value) -> String {
    sessions::workflow(&tool_context(cfg), &payload).expect("workflow")
}

fn invoke_sync(cfg: &Config) -> String {
    search::sync(&tool_context(cfg), &json!({})).expect("sync")
}

fn extract_workflow_session_id(result: &str) -> String {
    result
        .lines()
        .find_map(|line| line.strip_prefix("Created session:"))
        .map(str::trim)
        .map(str::to_string)
        .expect("Created session line")
}

fn yaml_string(frontmatter: &Frontmatter, key: &str) -> String {
    frontmatter
        .get(key)
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string()
}

fn yaml_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|part| part.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

fn session_uri(session_type: &str, session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/{session_type}/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn workflow_uri(session_id: &str) -> String {
    session_uri("workflow", session_id)
}

fn sequential_uri(session_id: &str) -> String {
    session_uri("sequential", session_id)
}

fn read_frontmatter(cfg: &Config, uri: &str) -> Frontmatter {
    read_markdown(uri, cfg)
        .expect("read markdown")
        .0
        .expect("frontmatter")
}

fn read_body(cfg: &Config, uri: &str) -> String {
    read_markdown(uri, cfg).expect("read markdown").1
}

fn create_thinking_session(cfg: &Config, status: &str) -> String {
    let suffix = 10_000 + (SESSION_COUNTER.fetch_add(1, Ordering::Relaxed) % 90_000);
    let session_id = format!("session-{}-{suffix}", Utc::now().timestamp_millis());
    let uri = sequential_uri(&session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Test Thinking Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "thoughtCount".to_string(),
        YamlValue::Number(YamlNumber::from(1)),
    );
    frontmatter.insert(
        "totalThoughts".to_string(),
        YamlValue::Number(YamlNumber::from(3)),
    );
    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Test Thinking Session\n\nThought 1: Working on [[hierarchical-state-machines]] implementation.\n",
        cfg,
    )
    .expect("write thinking session");
    session_id
}

fn update_thinking_session_status(cfg: &Config, session_id: &str, new_status: &str) {
    let uri = sequential_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, cfg).expect("read thinking session");
    let mut frontmatter = frontmatter.expect("thinking frontmatter");
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String(new_status.to_string()),
    );
    write_markdown(&uri, Some(&frontmatter), &content, cfg).expect("update thinking session");
}

fn create_legacy_workflow_session(cfg: &Config) -> String {
    let session_id = format!("workflow-{}", Utc::now().timestamp_millis());
    let uri = workflow_uri(&session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Legacy Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "queue".to_string(),
        YamlValue::Sequence(vec![YamlValue::String("wsup-alpha".to_string())]),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(YamlNumber::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(YamlNumber::from(0)),
    );
    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Workflow Session\n\nLegacy session content with [[backward-compatibility]] test.\n",
        cfg,
    )
    .expect("write legacy workflow session");
    session_id
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

fn write_raw_markdown(cfg: &Config, file_uri: &str, frontmatter: &Frontmatter, body: &str) {
    let path = uri_to_path(file_uri, &cfg.memory_path).expect("uri to path");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    let mut text = String::new();
    if !frontmatter.is_empty() {
        text.push_str("---\n");
        text.push_str(&serde_yaml::to_string(frontmatter).expect("yaml"));
        text.push_str("---\n\n");
    }
    text.push_str(body);
    fs::write(path, text).expect("write raw markdown");
}

fn seed_file_content(
    cfg: &Config,
    file_uri: &str,
    status: Option<&str>,
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
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(file_path) DO UPDATE SET
             title = excluded.title,
             content = excluded.content,
             file_md5 = excluded.file_md5,
             file_size = excluded.file_size,
             last_indexed = excluded.last_indexed,
             status = excluded.status",
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
    seed_file_content(
        cfg,
        file_uri,
        Some(status),
        body,
        &iso_minutes_ago(minutes_ago),
    );
}

fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - Duration::minutes(minutes_ago)).to_rfc3339()
}

fn read_frontmatter_status(cfg: &Config, file_uri: &str) -> String {
    read_markdown(file_uri, cfg)
        .expect("read markdown")
        .0
        .and_then(|values| {
            values
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_string)
        })
        .expect("status")
}

fn read_db_status(cfg: &Config, file_uri: &str) -> String {
    Connection::open(&cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT status FROM file_content WHERE file_path = ?1",
            params![file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("db status")
}

fn read_db_content(cfg: &Config, file_uri: &str) -> String {
    Connection::open(&cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT content FROM file_content WHERE file_path = ?1",
            params![file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("db content")
}

fn file_content_row_count(cfg: &Config) -> i64 {
    Connection::open(&cfg.database_path)
        .expect("open db")
        .query_row("SELECT COUNT(*) FROM file_content", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("row count")
}

fn create_normal_memory_file(
    cfg: &Config,
    relative_path: &str,
    content: &str,
) -> (PathBuf, String) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write memory file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

fn set_file_mtime_utc(path: &Path, instant: DateTime<Utc>) {
    set_file_mtime(
        path,
        FileTime::from_unix_time(instant.timestamp(), instant.timestamp_subsec_nanos()),
    )
    .expect("set mtime");
}

fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}

fn align_to_second_boundary() {
    let nanos = Utc::now().nanosecond();
    if nanos > 900_000_000 {
        thread::sleep(StdDuration::from_millis(150));
        return;
    }
    let wait = 1_000_000_000u32 - nanos + 20_000_000;
    thread::sleep(StdDuration::from_nanos(u64::from(wait)));
}

// T-W1-7: RTM FR-10.3
#[test]
fn workflow_start_writes_supervisor_frontmatter_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = invoke_workflow(&env.cfg, json!({"workflowId": "wsup-alpha"}));
    let session_id = extract_workflow_session_id(&result);
    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&session_id));

    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
}

// T-W1-7: RTM FR-10.1, FR-10.3, FR-10.4
#[test]
fn register_submachine_enters_waiting_state_without_advancing_step() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_result = invoke_workflow(&env.cfg, json!({"workflowId": "wsup-alpha"}));
    let workflow_session_id = extract_workflow_session_id(&workflow_result);
    let thinking_session_id = create_thinking_session(&env.cfg, "active");

    let step_before = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );

    let register_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting analysis with [[hierarchical-state-machines]] and [[sequential-thinking]].",
            "submachineSessionId": thinking_session_id,
        }),
    );

    assert!(register_result.contains("Registered submachine"));
    assert!(register_result.contains(&thinking_session_id));
    assert!(register_result.contains("waiting"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "waiting");
    assert_eq!(
        yaml_string(&frontmatter, "activeSubmachineType"),
        "sequential"
    );
    assert_eq!(
        yaml_string(&frontmatter, "activeSubmachineSessionId"),
        thinking_session_id
    );
    assert_eq!(yaml_usize(&frontmatter, "currentStep"), step_before);
}

// T-W1-7: RTM FR-10.4
#[test]
fn continue_while_waiting_on_active_submachine_blocks_and_does_not_advance_step() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine with [[state-machine]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let step_after_register = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );

    let block_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to advance but submachine is still running [[workflow]].",
        }),
    );

    assert!(block_result.contains("waiting on submachine"));
    assert!(block_result.contains(&thinking_session_id));
    assert!(!block_result.contains("Step 2/"));
    assert_eq!(
        yaml_usize(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "currentStep"
        ),
        step_after_register
    );
}

// T-W1-7: RTM FR-10.1, FR-10.3, FR-10.4
#[test]
fn continue_after_submachine_completed_clears_supervisor_fields_and_resumes() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering [[sequential-thinking]] submachine.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "completed");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming with [[hierarchical-state-machines]] after submachine completed.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(!resume_result.contains("ERROR"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
}

// T-W1-7: RTM FR-10.1
#[test]
fn continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine for [[state-machine]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "cancelled");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[sequential-thinking]] submachine was cancelled.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(resume_result.contains("Step 2/"));
    assert_eq!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        "running"
    );
}

// T-W1-7: RTM FR-10.2
#[test]
fn backward_compatibility_old_session_without_phase_field_proceeds_normally() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let session_id = create_legacy_workflow_session(&env.cfg);
    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&session_id));
    assert!(!frontmatter.contains_key("phase"));

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": session_id,
            "response": "Step 1 done with [[workflow]] and [[backward-compatibility]] verification.",
        }),
    );

    assert!(!result.contains("ERROR"));
    assert!(result.contains("Step 2/"));
}

// T-W1-7: RTM FR-10.1
#[test]
fn register_nonexistent_submachine_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let fake_session_id = "session-9999999999999-00001";

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering with [[hierarchical-state-machines]] context.",
            "submachineSessionId": fake_session_id,
        }),
    );

    assert!(result.contains("ERROR"));
    assert!(result.contains(fake_session_id));
    assert_ne!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        "waiting"
    );
}

// T-W1-7: RTM FR-10.3
#[test]
fn register_submachine_appends_note_to_session_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");

    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine with [[workflow]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Submachine Registered"));
    assert!(content.contains(&thinking_session_id));
    assert!(content.contains("sequential"));
}

// T-W1-7: RTM FR-10.3
#[test]
fn unblock_after_completion_appends_resume_note_to_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering with [[state-machine]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "completed");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after submachine with [[hierarchical-state-machines]] done.",
        }),
    );

    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Submachine Resumed"));
    assert!(content.contains("completed"));
}

// T-W1-7: RTM FR-10.1
#[test]
fn abandoned_submachine_unblocks_workflow() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine for [[hierarchical-state-machines]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "abandoned");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[sequential-thinking]] submachine was abandoned.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(!resume_result.contains("ERROR"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
}

// T-W1-7: RTM FR-10.1
#[test]
fn missing_submachine_file_unblocks_workflow() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine with [[state-machine]] and [[workflow]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let waiting_frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&waiting_frontmatter, "phase"), "waiting");

    let submachine_path = uri_to_path(&sequential_uri(&thinking_session_id), &env.cfg.memory_path)
        .expect("submachine path");
    assert!(submachine_path.exists());
    fs::remove_file(&submachine_path).expect("delete submachine file");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[hierarchical-state-machines]] submachine was deleted.",
        }),
    );

    assert!(!resume_result.contains("ERROR"));
    assert!(!resume_result.contains("waiting on submachine"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
    assert!(content.contains("Submachine Cleared"));
    assert!(content.contains(&thinking_session_id));
    assert!(content.contains("deleted or expired"));
}

// T-W1-7: RTM FR-10.3, FR-10.4
#[test]
fn response_persisted_during_register() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    let response =
        "Analysis complete for [[hierarchical-state-machines]] and [[workflow]] at step boundary.";
    let step_before = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );

    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": response,
            "submachineSessionId": thinking_session_id,
        }),
    );

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Analysis complete for"));
    assert!(content.contains("[[hierarchical-state-machines]]"));
    assert_eq!(yaml_usize(&frontmatter, "currentStep"), step_before);
    assert_eq!(yaml_string(&frontmatter, "phase"), "waiting");
}

// T-W1-7: RTM FR-10.3
#[test]
fn register_path_enforces_wikilink_validation_on_response() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "No double bracket concepts in this response at all.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    assert!(result.contains("ERROR"));
    assert!(result.contains("[[WikiLink]]"));
    assert_ne!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        "waiting"
    );
}

// T-W1-7: RTM FR-10.1
#[test]
fn re_registration_cycle_works_across_multiple_submachines() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));

    let thinking_session_id_a = create_thinking_session(&env.cfg, "active");
    let step_before_a = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting [[hierarchical-state-machines]] analysis with submachine A.",
            "submachineSessionId": thinking_session_id_a,
        }),
    );

    let frontmatter_after_reg_a = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_reg_a, "phase"), "waiting");
    assert_eq!(
        yaml_usize(&frontmatter_after_reg_a, "currentStep"),
        step_before_a
    );

    update_thinking_session_status(&env.cfg, &thinking_session_id_a, "completed");
    let resume_result_a = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming [[workflow]] after submachine A completed — advancing step.",
        }),
    );
    assert!(!resume_result_a.contains("waiting on submachine"));
    assert!(resume_result_a.contains("Step 2/"));

    let frontmatter_after_resume_a =
        read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_resume_a, "phase"), "running");
    assert_eq!(
        yaml_string(&frontmatter_after_resume_a, "activeSubmachineSessionId"),
        ""
    );
    let step_after_resume_a = yaml_usize(&frontmatter_after_resume_a, "currentStep");

    let thinking_session_id_b = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting [[sequential-thinking]] submachine B on step 2.",
            "submachineSessionId": thinking_session_id_b,
        }),
    );

    let frontmatter_after_reg_b = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_reg_b, "phase"), "waiting");
    assert_eq!(
        yaml_string(&frontmatter_after_reg_b, "activeSubmachineSessionId"),
        thinking_session_id_b
    );
    assert_eq!(
        yaml_usize(&frontmatter_after_reg_b, "currentStep"),
        step_after_resume_a
    );

    update_thinking_session_status(&env.cfg, &thinking_session_id_b, "completed");
    let resume_result_b = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming [[hierarchical-state-machines]] after submachine B completed — advancing step.",
        }),
    );
    assert!(!resume_result_b.contains("waiting on submachine"));
    assert!(resume_result_b.contains("Step 3/"));

    let frontmatter_after_resume_b =
        read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_resume_b, "phase"), "running");
    assert_eq!(
        yaml_string(&frontmatter_after_resume_b, "activeSubmachineSessionId"),
        ""
    );
    assert_eq!(
        yaml_string(&frontmatter_after_resume_b, "activeSubmachineType"),
        ""
    );
}

// T-W1-7: RTM FR-10.4
#[test]
fn double_registration_while_waiting_is_blocked() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let first_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering first [[hierarchical-state-machines]] submachine.",
            "submachineSessionId": first_session_id,
        }),
    );

    let after_first = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_first, "phase"), "waiting");
    assert_eq!(
        yaml_string(&after_first, "activeSubmachineSessionId"),
        first_session_id
    );

    let second_session_id = create_thinking_session(&env.cfg, "active");
    let second_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Attempting to register second [[sequential-thinking]] submachine while waiting.",
            "submachineSessionId": second_session_id,
        }),
    );

    assert!(second_result.contains("waiting on submachine"));
    let after_second = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_second, "phase"), "waiting");
    assert_eq!(
        yaml_string(&after_second, "activeSubmachineSessionId"),
        first_session_id
    );
}

// T-W1-7: RTM FR-10.4
#[test]
fn status_parameter_while_waiting_is_blocked() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering [[hierarchical-state-machines]] submachine before attempting completion.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let after_register = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_register, "phase"), "waiting");

    let complete_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to complete [[workflow]] while submachine is active.",
            "status": "completed",
            "conclusion": "Attempting completion with [[hierarchical-state-machines]] conclusion.",
        }),
    );
    assert!(complete_result.contains("waiting on submachine"));

    let cancel_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to cancel [[workflow]] while submachine is active.",
            "status": "cancelled",
        }),
    );
    assert!(cancel_result.contains("waiting on submachine"));

    let after_attempts = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_attempts, "status"), "active");
    assert_eq!(yaml_string(&after_attempts, "phase"), "waiting");
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_stale_workflow_session_marks_status_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/stale-workflow-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Test Session\n\nContent [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_stale_workflow_session_appends_abandoned_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/stale-workflow-section-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Test Session\n\nContent [[Session Cleanup]]",
    );

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert!(read_body(&env.cfg, file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_stale_sequential_session_marks_status_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/stale-sequential-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Sequential Session\n\nContent [[Session Cleanup]]",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_stale_sequential_session_appends_abandoned_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/stale-sequential-section-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Sequential Session\n\nContent [[Session Cleanup]]",
    );

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert!(read_body(&env.cfg, file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_recent_active_session_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/recent-session-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        30,
        "# Recent Session\n\nContent [[Session Cleanup]]",
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_null_type_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://notes/null-type-test";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_raw_markdown(&env.cfg, file_uri, &frontmatter, "# Note\n\nContent");
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Note\n\nContent",
        &iso_minutes_ago(130),
    );
    let path = uri_to_path(file_uri, &env.cfg.memory_path).expect("uri path");
    let before_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    let after_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(abandoned, 0);
    assert_eq!(before_mtime, after_mtime);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_memory_type_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://notes/memory-type-test";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_raw_markdown(&env.cfg, file_uri, &frontmatter, "# Memory Note\n\nContent");
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Memory Note\n\nContent",
        &iso_minutes_ago(130),
    );
    let path = uri_to_path(file_uri, &env.cfg.memory_path).expect("uri path");
    let before_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    let after_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(abandoned, 0);
    assert_eq!(before_mtime, after_mtime);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_already_abandoned_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/already-abandoned-test";
    create_session(
        &env.cfg,
        file_uri,
        "abandoned",
        130,
        "# Abandoned Session\n\nContent [[Session Cleanup]]",
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_missing_modified_field_no_exception_and_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/missing-modified-test";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_raw_markdown(
        &env.cfg,
        file_uri,
        &frontmatter,
        "# Session\n\nContent [[Session Cleanup]]",
    );
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Session\n\nContent [[Session Cleanup]]",
        &iso_minutes_ago(30),
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_invalid_date_in_modified_no_exception_and_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/invalid-date-test";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String("not-a-date".to_string()),
    );
    write_raw_markdown(
        &env.cfg,
        file_uri,
        &frontmatter,
        "# Session\n\nContent [[Session Cleanup]]",
    );
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Session\n\nContent [[Session Cleanup]]",
        &iso_minutes_ago(30),
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_stale_session_returns_true() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/stale-return-true";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Stale Workflow\n\nSome content [[Session Cleanup]].",
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_stale_session_updates_frontmatter_status() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/stale-updates-status";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Stale Workflow\n\nSome content [[Session Cleanup]].",
    );

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_stale_session_appends_abandoned_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/stale-appends-section";
    let original = "# Stale Workflow\n\nSome content [[Session Cleanup]].";
    create_session(&env.cfg, file_uri, "active", 130, original);

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert!(read_body(&env.cfg, file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_recent_session_returns_false() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/recent-returns-false";
    let original = "# Recent Session\n\nSome content [[Session Cleanup]].";
    create_session(&env.cfg, file_uri, "active", 30, original);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_body(&env.cfg, file_uri), original);
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_recent_session_frontmatter_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/recent-frontmatter-unchanged";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        30,
        "# Recent Session\n\nSome content [[Session Cleanup]].",
    );

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_exactly_at_threshold_returns_false() {
    let _guard = TEST_MUTEX.lock();
    let _env_guard = SessionCleanupEnvGuard::set_abandonment_minutes("5");
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/exact-threshold";
    write_session_markdown(
        &env.cfg,
        file_uri,
        "active",
        "# Boundary Session\n\nContent [[Session Cleanup]].",
    );
    align_to_second_boundary();
    let anchor = Utc::now().with_nanosecond(0).expect("truncate nanos");
    let exact_threshold = (anchor - Duration::minutes(5)).to_rfc3339();
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Boundary Session\n\nContent [[Session Cleanup]].",
        &exact_threshold,
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
}

// T-W1-7: RTM FR-17.7
#[test]
fn try_mark_abandoned_from_last_indexed_stale_session_preserves_original_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/preserves-original-content";
    let original = "# My Workflow\n\nImportant details about the work.\n\n## Step 1\n\nDo something. [[Session Cleanup]]\n";
    create_session(&env.cfg, file_uri, "active", 130, original);

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    let updated = read_body(&env.cfg, file_uri);
    assert!(updated.starts_with(original));
}

// T-W1-7: RTM FR-14.6
#[test]
fn sync_stale_workflow_session_marked_abandoned_even_when_mtime_guard_would_skip_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let session_id = create_workflow_session_fixture(&env.cfg, "workflow", "active");
    let file_uri = workflow_uri(&session_id);
    let path = uri_to_path(&file_uri, &env.cfg.memory_path).expect("path");

    invoke_sync(&env.cfg);

    let stale = Utc::now() - Duration::minutes(130);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, Some("active"));

    let last_indexed_db = Connection::open(&env.cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            params![&file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("last indexed");
    assert_eq!(last_indexed_db, last_indexed);

    invoke_sync(&env.cfg);

    assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
    assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-14.6
#[test]
fn sync_stale_sequential_session_marked_abandoned_even_when_mtime_guard_would_skip_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let session_id = create_workflow_session_fixture(&env.cfg, "sequential", "active");
    let file_uri = sequential_uri(&session_id);
    let path = uri_to_path(&file_uri, &env.cfg.memory_path).expect("path");

    invoke_sync(&env.cfg);

    let stale = Utc::now() - Duration::minutes(130);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, Some("active"));

    let last_indexed_db = Connection::open(&env.cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            params![&file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("last indexed");
    assert_eq!(last_indexed_db, last_indexed);

    invoke_sync(&env.cfg);

    assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
    assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-14.6
#[test]
fn sync_does_not_rewrite_normal_memory_file_solely_due_to_age_mtime_guard_remains() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let (path, file_uri) = create_normal_memory_file(
        &env.cfg,
        "notes/cleanup-001-1/note.md",
        "# Normal Note\n\nThis is a normal file, not a thinking session. [[Cleanup Note]]\n",
    );

    invoke_sync(&env.cfg);

    let stale = Utc::now() - Duration::days(7);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, None);

    let before_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let before_text = fs::read_to_string(&path).expect("read before");

    invoke_sync(&env.cfg);

    let after_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let after_text = fs::read_to_string(&path).expect("read after");
    assert_eq!(before_mtime, after_mtime);
    assert_eq!(before_text, after_text);
}

fn create_workflow_session_fixture(cfg: &Config, session_type: &str, status: &str) -> String {
    let millis = Utc::now().timestamp_millis();
    let session_id = match session_type {
        "workflow" => format!("workflow-{millis}"),
        "sequential" => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as u64;
            format!("session-{millis}-{}", 10_000 + (nanos % 90_000))
        }
        other => panic!("unsupported session type: {other}"),
    };
    let uri = session_uri(session_type, &session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Test {session_type} session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String(session_type.to_string()),
    );
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    let body = if session_type == "workflow" {
        "# Workflow Session\n\n## Step 1\n\nOld [[Session Cleanup]] workflow\n"
    } else {
        "# Session\n\n## Thought 1\nOld [[Session Cleanup]] session\n"
    };
    write_markdown(&uri, Some(&frontmatter), body, cfg).expect("write fixture session");
    session_id
}

fn update_last_indexed_and_status(
    cfg: &Config,
    file_uri: &str,
    last_indexed: &str,
    status: Option<&str>,
) {
    Connection::open(&cfg.database_path)
        .expect("open db")
        .execute(
            "UPDATE file_content SET last_indexed = ?2, status = ?3 WHERE file_path = ?1",
            params![file_uri, last_indexed, status],
        )
        .expect("update indexed state");
}
