// T-12.7: RTM NFR-1
#[allow(clippy::duplicate_mod)]
#[path = "workflow_supervisor/support.rs"]
mod support;
#[allow(clippy::duplicate_mod)]
#[path = "workflow_supervisor/sync_support.rs"]
mod sync_support;

use chrono::{Duration, Timelike, Utc};
use maenifold_core::sessions;
use support::*;
use sync_support::align_to_second_boundary;

// T-W1-7: RTM FR-17.7 — temporal guard: recent session remains active and unchanged
#[test]
fn recent_session_before_abandonment_threshold_remains_active_and_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/recent-threshold-session";
    let original = "# Recent Session\n\nContent [[Session Cleanup]]";
    create_session(&env.cfg, file_uri, "active", 30, original);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_body(&env.cfg, file_uri), original);
}

// T-W1-7: RTM FR-17.7
#[test]
fn session_at_exact_abandonment_boundary_is_not_marked_abandoned() {
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
fn stale_session_abandonment_appends_section_without_corrupting_original_body() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/preserves-original-content";
    let original = "# My Workflow\n\nImportant details about the work.\n\n## Step 1\n\nDo something. [[Session Cleanup]]\n";
    create_session(&env.cfg, file_uri, "active", 130, original);

    sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    let updated = read_body(&env.cfg, file_uri);
    assert!(updated.starts_with(original));
}
