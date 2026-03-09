use chrono::{TimeDelta, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::tools::{
    self, RecentActivityRecord, ToolContext, collect_recent_activity_records,
    format_recent_activity_report, recent_activity_determine_file_type,
    recent_activity_extract_session_id, recent_activity_extract_snippet,
};
use maenifold_core::{AssetManager, Config, GraphDatabase, MaenifoldError, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
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

fn isolated_env(initialize_database: bool) -> TestEnv {
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

fn empty_db_env() -> TestEnv {
    isolated_env(false)
}

fn invoke_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("invoke Sync")
}

fn invoke_recent_activity(payload: serde_json::Value) -> Result<String, MaenifoldError> {
    ToolRegistry::global().try_invoke("RecentActivity", &payload, OutputFormat::Markdown)
}

fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

fn rendered_item_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_start().starts_with("**"))
        .count()
}

fn seed_sequential_file(cfg: &Config, session_id: &str, thought_content: &str) {
    let seq_dir = cfg
        .sequential_path
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&seq_dir).expect("sequential dir");
    let content = format!("# {session_id}\n\n{thought_content}\n");
    fs::write(seq_dir.join(format!("{session_id}.md")), content).expect("sequential file");
}

fn seed_memory_file(cfg: &Config, relative_path: &str, content: &str) {
    let full_path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(full_path.parent().expect("parent")).expect("memory dir");
    fs::write(full_path, content).expect("memory file");
}

fn seed_chat_file(cfg: &Config, file_name: &str, content: &str) {
    let chat_dir = cfg
        .memory_path
        .join("chat")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&chat_dir).expect("chat dir");
    fs::write(chat_dir.join(file_name), content).expect("chat file");
}

fn set_last_indexed(cfg: &Config, file_name_fragment_without_extension: &str, timestamp_utc: &str) {
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

fn sample_record(
    file_path: &str,
    title: &str,
    last_indexed: &str,
    content: &str,
    status: &str,
) -> RecentActivityRecord {
    RecentActivityRecord {
        file_path: file_path.to_string(),
        title: title.to_string(),
        last_indexed: last_indexed.to_string(),
        content: content.to_string(),
        status: status.to_string(),
    }
}

// T-2.3: RTM FR-4 / RTM-020
#[test]
fn collect_recent_activity_returns_all_files_when_filter_is_null() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-all-test",
        "## Thought 1/3\nThis is thought one.",
    );
    seed_memory_file(
        &env.cfg,
        "notes/note-all-test.md",
        "# Note All Test\n\n## Section\nSome content.",
    );
    invoke_sync();

    let results = collect_recent_activity_records(&env.cfg, None, None, 50).expect("collect");
    let paths = results
        .iter()
        .map(|row| row.file_path.as_str())
        .collect::<Vec<_>>();

    assert!(
        results.len() >= 2,
        "expected at least two results: {results:?}"
    );
    assert!(paths.iter().any(|path| path.contains("session-all-test")));
    assert!(paths.iter().any(|path| path.contains("note-all-test")));
}

// T-2.3: RTM FR-4 / RTM-021
#[test]
fn collect_recent_activity_filter_thinking_returns_only_thinking_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-thinking-filter",
        "## Thought 1/2\nFirst thought.",
    );
    seed_memory_file(
        &env.cfg,
        "notes/note-thinking-filter.md",
        "# Note\n\n## Section\nNot thinking.",
    );
    invoke_sync();

    let results =
        collect_recent_activity_records(&env.cfg, None, Some("thinking"), 50).expect("collect");
    let paths = results
        .iter()
        .map(|row| row.file_path.as_str())
        .collect::<Vec<_>>();

    assert!(
        !results.is_empty(),
        "expected thinking results: {results:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.contains("session-thinking-filter"))
    );
    assert!(
        !paths
            .iter()
            .any(|path| path.contains("note-thinking-filter"))
    );
}

// T-2.3: RTM FR-4 / RTM-022
#[test]
fn collect_recent_activity_filter_memory_excludes_thinking_and_chat_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-mem-filter",
        "## Thought 1/2\nThought content.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/note-mem-filter.md",
        "# Doc Note\n\n## Section\nDoc content.",
    );
    seed_chat_file(
        &env.cfg,
        "chat-memory-filter.md",
        "# Chat\n\n## Message\nAssumption.",
    );
    invoke_sync();

    let results =
        collect_recent_activity_records(&env.cfg, None, Some("memory"), 50).expect("collect");
    let paths = results
        .iter()
        .map(|row| row.file_path.as_str())
        .collect::<Vec<_>>();

    assert!(!paths.iter().any(|path| path.contains("session-mem-filter")));
    assert!(paths.iter().any(|path| path.contains("note-mem-filter")));
    assert!(!paths.iter().any(|path| path.contains("chat-memory-filter")));
}

// T-2.3: RTM FR-4 / RTM-023
#[test]
fn collect_recent_activity_filter_assumptions_returns_chat_path_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_chat_file(
        &env.cfg,
        "chat-assumptions-filter.md",
        "# Chat Session\n\n## Message\nHello from chat.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/regular-note.md",
        "# Note\n\n## Intro\nRegular content.",
    );
    invoke_sync();

    let results =
        collect_recent_activity_records(&env.cfg, None, Some("assumptions"), 50).expect("collect");
    let paths = results
        .iter()
        .map(|row| row.file_path.as_str())
        .collect::<Vec<_>>();

    assert!(
        paths
            .iter()
            .any(|path| path.contains("chat-assumptions-filter"))
    );
    assert!(!paths.iter().any(|path| path.contains("regular-note")));
}

// T-2.3: RTM FR-4 / RTM-024
#[test]
fn collect_recent_activity_timespan_filter_excludes_old_entries() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-recent",
        "## Thought 1/2\nRecent thought.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/old-note.md",
        "# Old Note\n\n## Intro\nOld content.",
    );
    invoke_sync();
    set_last_indexed(&env.cfg, "old-note", "2020-01-01 00:00:00");

    let results = collect_recent_activity_records(&env.cfg, Some(TimeDelta::hours(24)), None, 50)
        .expect("collect");
    let paths = results
        .iter()
        .map(|row| row.file_path.as_str())
        .collect::<Vec<_>>();

    assert!(!paths.iter().any(|path| path.contains("old-note")));
    assert!(paths.iter().any(|path| path.contains("session-recent")));
}

// T-2.3: RTM FR-4 / RTM-026
#[test]
fn collect_recent_activity_limit_respects_max_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    for index in 0..5 {
        seed_memory_file(
            &env.cfg,
            &format!("docs/limit-note-{index}.md"),
            &format!("# Note {index}\n\n## Section\nContent {index}."),
        );
    }
    invoke_sync();

    let results = collect_recent_activity_records(&env.cfg, None, None, 2).expect("collect");
    assert_eq!(results.len(), 2);
}

// T-2.3: RTM FR-4 / RTM-027
#[test]
fn collect_recent_activity_returns_error_when_no_database_exists() {
    let _guard = TEST_MUTEX.lock();
    let env = empty_db_env();

    let err = collect_recent_activity_records(&env.cfg, None, None, 10).expect_err("missing db");
    match err {
        MaenifoldError::Message(message) => assert!(message.contains("No database found")),
        other => panic!("unexpected error: {other:?}"),
    }
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_empty_input_returns_empty_string() {
    assert_eq!(recent_activity_extract_snippet("", 100), "");
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_short_content_returns_full_content() {
    let content = "This is a short sentence.";
    assert_eq!(recent_activity_extract_snippet(content, 200), content);
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_long_content_truncates_with_ellipsis() {
    let long_content = "A".repeat(200);
    let result = recent_activity_extract_snippet(&long_content, 50);
    assert_eq!(result.chars().count(), 50);
    assert!(result.ends_with("..."));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_bullet_only_content_falls_back_to_all_lines() {
    let content = "* Bullet one\n* Bullet two\n* Bullet three";
    let result = recent_activity_extract_snippet(content, 1000);
    assert!(!result.is_empty());
    assert!(result.contains("Bullet"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_mixed_content_skips_bullet_lines() {
    let content = "* Metadata bullet\nActual prose content here.";
    let result = recent_activity_extract_snippet(content, 1000);
    assert!(result.contains("prose content"));
    assert!(!result.contains("Metadata bullet"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_uses_default_length_when_minus_one() {
    let long_content = "X".repeat(2000);
    let result = recent_activity_extract_snippet(&long_content, -1);
    assert!(
        result.chars().count() <= 1000,
        "unexpected len: {}",
        result.chars().count()
    );
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_standard_sequential_path_extracts_id() {
    let id =
        recent_activity_extract_session_id("memory://thinking/sequential/2026/02/session-12345.md");
    assert_eq!(id, "session-12345");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_simple_memory_path_extracts_filename() {
    let id = recent_activity_extract_session_id("memory://docs/my-note.md");
    assert_eq!(id, "my-note");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_chat_path_extracts_filename() {
    let id = recent_activity_extract_session_id("memory://chat/2026/02/chat-session.md");
    assert_eq!(id, "chat-session");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_sequential_path_returns_sequential() {
    let file_type =
        recent_activity_determine_file_type("memory://thinking/sequential/2026/02/session-abc.md");
    assert_eq!(file_type, "sequential");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_workflow_path_returns_workflow() {
    let file_type =
        recent_activity_determine_file_type("memory://thinking/workflow/2026/02/wf-session.md");
    assert_eq!(file_type, "workflow");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_memory_path_returns_memory() {
    let file_type = recent_activity_determine_file_type("memory://docs/my-note.md");
    assert_eq!(file_type, "memory");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_chat_path_returns_memory() {
    let file_type = recent_activity_determine_file_type("memory://chat/2026/02/assumptions.md");
    assert_eq!(file_type, "memory");
}

// T-2.3: RTM FR-4 / RTM-027
#[test]
fn format_activity_report_empty_results_returns_no_activity_message() {
    let output = format_recent_activity_report(Vec::new(), false);
    assert_eq!(output, "No recent activity found");
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_sequential_entry_includes_expected_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-001.md",
            "Test Sequential Session",
            "2026-02-18 10:00:00",
            "# Test Sequential Session\n\n## Thought 1/2\nFirst thought content here.\n\n## Thought 2/2\nSecond thought content here.",
            "active",
        )],
        false,
    );

    assert!(output.contains("# Recent Activity"));
    assert!(output.contains("session-fmt-001"));
    assert!(output.contains("(sequential)"));
    assert!(output.contains("Thoughts:"));
    assert!(output.contains("Status: active"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_sequential_entry_include_content_true_shows_h2_content_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-content.md",
            "Content Session",
            "2026-02-18 11:00:00",
            "# Content Session\n\n## Thought 1/2\nFirst prose.\n\n## Thought 2/2\nSecond prose.",
            "active",
        )],
        true,
    );

    assert!(output.contains("First H2:"));
    assert!(output.contains("First H2 Content:"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_sequential_entry_include_content_false_shows_snippet() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-snippet.md",
            "Snippet Session",
            "2026-02-18 12:00:00",
            "# Snippet Session\n\n## Thought 1/3\nThought content for snippet.\n\n## Thought 2/3\nSecond thought.\n\n## Thought 3/3\nThird thought.",
            "active",
        )],
        false,
    );

    assert!(output.contains("First:"));
    assert!(!output.contains("First H2:"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_workflow_entry_includes_steps_and_status() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/workflow/2026/02/wf-fmt-001.md",
            "Workflow Fmt Test",
            "2026-02-18 13:00:00",
            "# Workflow Fmt Test\n\n## Step 1: Init\nInitialization response.\n\n## Response\nFinal result.",
            "active",
        )],
        false,
    );

    assert!(output.contains("wf-fmt-001"));
    assert!(output.contains("(workflow)"));
    assert!(output.contains("Steps:"));
    assert!(output.contains("Status: active"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_memory_entry_includes_title_and_sections() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://docs/memory-fmt-test.md",
            "Memory Format Test",
            "2026-02-18 14:00:00",
            "# Memory Format Test\n\n## Overview\nOverview content.\n\n## Details\nDetailed content here.",
            "active",
        )],
        false,
    );

    assert!(output.contains("memory-fmt-test"));
    assert!(output.contains("(memory)"));
    assert!(output.contains("Title: Memory Format Test"));
    assert!(output.contains("Sections:"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_memory_entry_include_content_true_shows_h2_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://docs/mem-content-test.md",
            "Memory Content Test",
            "2026-02-18 15:00:00",
            "# Memory Content Test\n\n## First Section\nFirst section content.\n\n## Last Section\nLast section content.",
            "active",
        )],
        true,
    );

    assert!(output.contains("First H2: First Section"));
    assert!(output.contains("First H2 Content:"));
    assert!(output.contains("Last H2: Last Section"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_multiple_entries_all_entities_present() {
    let output = format_recent_activity_report(
        vec![
            sample_record(
                "memory://thinking/sequential/2026/02/session-multi-a.md",
                "Session A",
                "2026-02-18 16:00:00",
                "# Session A\n\n## Thought 1/1\nA thought.",
                "active",
            ),
            sample_record(
                "memory://docs/note-multi-b.md",
                "Note B",
                "2026-02-18 15:00:00",
                "# Note B\n\n## Section B\nSome note content.",
                "active",
            ),
        ],
        false,
    );

    assert!(output.contains("session-multi-a"));
    assert!(output.contains("note-multi-b"));
    assert!(output.contains("(sequential)"));
    assert!(output.contains("(memory)"));
}

// T-2.3: RTM FR-4 / RTM-024
#[test]
fn recent_activity_negative_timespan_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    let context = markdown_context(&env.cfg);

    let err = tools::recent_activity(&context, &json!({ "timespan": "-01:00:00" }))
        .expect_err("negative timespan must error");
    match err {
        MaenifoldError::InvalidArgument(message) => {
            assert!(message.contains("timespan must be positive"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// T-2.3: RTM FR-4 / RTM-027
#[test]
fn recent_activity_no_database_returns_no_database_error() {
    let _guard = TEST_MUTEX.lock();
    let env = empty_db_env();
    let context = markdown_context(&env.cfg);

    let err = tools::recent_activity(&context, &json!({})).expect_err("missing db must error");
    match err {
        MaenifoldError::Message(message) => assert!(message.contains("No database found")),
        other => panic!("unexpected error: {other:?}"),
    }
}

// T-2.3: RTM FR-4 / RTM-028
#[test]
fn recent_activity_learn_mode_returns_help_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    let context = markdown_context(&env.cfg);

    let result = tools::recent_activity(&context, &json!({ "learn": true })).expect("learn");
    assert!(!result.is_empty());
    assert!(result.len() > 20);
    assert!(result.contains("RecentActivity"));
}

// T-2.3: RTM FR-4 / RTM-020
#[test]
fn recent_activity_normal_query_returns_formatted_report() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-e2e-001",
        "## Thought 1/2\nFirst thought.\n\n## Thought 2/2\nSecond thought.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/e2e-note.md",
        "# End-To-End Note\n\n## Section One\nNote content.",
    );
    invoke_sync();

    let result = invoke_recent_activity(json!({ "limit": 20 })).expect("recent activity");
    assert!(result.contains("# Recent Activity"));
    assert!(result.contains("session-e2e-001"));
    assert!(result.contains("e2e-note"));
}

// T-2.3: RTM FR-4 / RTM-021
#[test]
fn recent_activity_filter_thinking_excludes_memory_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_sequential_file(
        &env.cfg,
        "session-e2e-filter",
        "## Thought 1/1\nSingle thought.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/excluded-note.md",
        "# Excluded Note\n\n## Section\nMust not appear.",
    );
    invoke_sync();

    let result = invoke_recent_activity(json!({ "filter": "thinking", "limit": 20 }))
        .expect("recent activity");
    assert!(result.contains("session-e2e-filter"));
    assert!(!result.contains("excluded-note"));
}

// T-2.3: RTM FR-4 / RTM-026
#[test]
fn recent_activity_limit_returns_at_most_limit_entries() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    for index in 0..4 {
        seed_memory_file(
            &env.cfg,
            &format!("docs/limit-e2e-{index}.md"),
            &format!("# Limit Note {index}\n\n## S\nContent."),
        );
    }
    invoke_sync();

    let result = invoke_recent_activity(json!({ "limit": 2 })).expect("recent activity");
    assert_eq!(
        rendered_item_count(&result),
        2,
        "unexpected output: {result}"
    );
}

// T-2.3: RTM FR-4 / RTM-024
#[test]
fn recent_activity_timespan_filter_excludes_old_entries_end_to_end() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env(true);
    seed_memory_file(
        &env.cfg,
        "docs/e2e-old-note.md",
        "# Old E2E Note\n\n## Intro\nOld content.",
    );
    seed_memory_file(
        &env.cfg,
        "docs/e2e-recent-note.md",
        "# Recent E2E Note\n\n## Intro\nRecent content.",
    );
    invoke_sync();
    set_last_indexed(&env.cfg, "e2e-old-note", "2019-06-01 00:00:00");

    let result = invoke_recent_activity(json!({
        "timespan": "24.00:00:00",
        "limit": 20
    }))
    .expect("recent activity");

    assert!(!result.contains("e2e-old-note"));
    assert!(result.contains("e2e-recent-note"));
}
