#[path = "session/session_cleanup_support.rs"]
mod session_cleanup_support;

use session_cleanup_support::{
    TEST_MUTEX, cleanup_abandoned_sessions, create_session, file_content_row_count, isolated_env,
    read_body, read_db_status, read_frontmatter_status,
};

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

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

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

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

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

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 0);
    assert_eq!(file_content_row_count(&env.cfg), 0);
}
