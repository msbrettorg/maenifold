// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;
#[path = "workflow_supervisor/sync_support.rs"]
mod sync_support;

use chrono::{Duration, Timelike, Utc};
use maenifold_core::sessions;
use support::*;
use sync_support::align_to_second_boundary;

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
