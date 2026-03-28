// T-12.7: RTM NFR-1
#[allow(clippy::duplicate_mod)]
#[path = "workflow_supervisor/support.rs"]
mod support;

use std::fs;

use maenifold_core::markdown::Frontmatter;
use maenifold_core::sessions;
use serde_yaml::Value as YamlValue;
use support::*;

// T-W1-7: RTM FR-17.7 — stale workflow session: count, status mutation, body append
#[test]
fn handle_session_cleanup_stale_workflow_session_is_abandoned() {
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
    assert!(
        read_body(&env.cfg, file_uri).contains("Session Abandoned"),
        "body should contain abandoned notice"
    );
}

// T-W1-7: RTM FR-17.7 — stale sequential session: count, status mutation, body append
#[test]
fn handle_session_cleanup_stale_sequential_session_is_abandoned() {
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
    assert!(
        read_body(&env.cfg, file_uri).contains("Session Abandoned"),
        "body should contain abandoned notice"
    );
}

// T-W1-7: RTM FR-17.7 — temporal guard: recent sessions are not abandoned
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

// T-W1-7: RTM FR-17.7 — URI path exclusion: files outside memory://thinking/ are not touched
#[test]
fn handle_session_cleanup_non_thinking_path_is_not_touched() {
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
    let path = path_for_uri(&env.cfg, file_uri);
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

// T-W1-7: RTM FR-17.7 — idempotency: re-cleanup of already-abandoned session is a no-op
#[test]
fn handle_session_cleanup_already_abandoned_session_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/idempotent-cleanup-test";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        130,
        "# Test Session\n\nContent [[Session Cleanup]]",
    );

    // First cleanup marks the session as abandoned
    let first_abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("first cleanup");
    assert_eq!(first_abandoned, 1);

    // Second cleanup should be a no-op
    let second_abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("second cleanup");
    assert_eq!(
        second_abandoned, 0,
        "already-abandoned session should not be re-abandoned"
    );

    // Body should contain exactly one "Session Abandoned" section
    let body = read_body(&env.cfg, file_uri);
    let notice_count = body.matches("Session Abandoned").count();
    assert_eq!(
        notice_count, 1,
        "expected exactly one abandoned notice, found {notice_count}"
    );
}
