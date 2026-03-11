#[path = "recent_activity_pipeline/support.rs"]
mod recent_activity_pipeline_support;

use chrono::TimeDelta;
use maenifold_core::MaenifoldError;
use recent_activity_pipeline_support::*;

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
