#[path = "recent_activity_pipeline/support.rs"]
mod recent_activity_pipeline_support;

use maenifold_core::MaenifoldError;
use maenifold_core::tools;
use recent_activity_pipeline_support::*;
use serde_json::json;

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
    assert!(!result.is_empty(), "learn mode returned empty string");
    assert!(
        result.contains("RecentActivity"),
        "help doc missing tool name header: {result}"
    );
    assert!(
        result.contains("## Parameters"),
        "help doc missing Parameters section: {result}"
    );
    assert!(
        result.contains("timespan"),
        "help doc missing timespan parameter: {result}"
    );
    assert!(
        result.contains("filter"),
        "help doc missing filter parameter: {result}"
    );
    assert!(
        result.contains("limit"),
        "help doc missing limit parameter: {result}"
    );
}

// T-2.3: RTM FR-4 / RTM-020
#[test]
fn recent_activity_pipeline_sync_produces_output_for_both_file_types() {
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
