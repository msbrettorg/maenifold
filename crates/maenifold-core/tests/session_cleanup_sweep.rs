#[path = "session/session_cleanup_support.rs"]
mod session_cleanup_support;

use session_cleanup_support::{
    TEST_MUTEX, cleanup_abandoned_sessions, create_pipeline_session, create_session,
    file_content_row_count, isolated_env, read_body, read_db_content, read_db_status,
    read_frontmatter_status,
};

// T-4.5b: RTM FR-4.2
#[test]
fn completed_and_cancelled_sessions_are_not_touched_regardless_of_age() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let completed_uri = "memory://thinking/sequential/2026/03/09/session-completed";
    let cancelled_uri = "memory://thinking/workflow/2026/03/09/workflow-cancelled";
    let abandoned_uri = "memory://thinking/sequential/2026/03/09/session-already-abandoned";
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
    // An already-abandoned session must also be immune to re-processing.
    // All three terminal statuses (completed, cancelled, abandoned) are covered.
    create_session(
        &env.cfg,
        abandoned_uri,
        "abandoned",
        600,
        "# Session\n\n## Thought 1\nAlready abandoned [[Session Cleanup]]",
    );

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 0);
    for (file_uri, expected_status) in [
        (completed_uri, "completed"),
        (cancelled_uri, "cancelled"),
        (abandoned_uri, "abandoned"),
    ] {
        assert_eq!(read_frontmatter_status(&env.cfg, file_uri), expected_status);
        assert_eq!(read_db_status(&env.cfg, file_uri), expected_status);
        let body = read_body(&env.cfg, file_uri);
        assert!(
            !body.contains("## Session Abandoned"),
            "{expected_status} session should not receive abandon notice, body: {body}"
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
    // T-0.3/NFR-4: The mtime/ctime guard (`should_preserve_recent_filesystem_edit`) checks
    // whether ctime lags mtime. For freshly written fixtures ctime ≈ mtime, so the guard
    // correctly allows abandonment. A minutes_ago of 5 keeps this session well inside the
    // 30-minute default TTL even on slow CI machines.
    create_session(
        &env.cfg,
        recent_active,
        "active",
        5,
        "# Session\n\n## Thought 1\nRecent [[Session Cleanup]] session",
    );

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 2);

    // Verify both old sessions are abandoned in frontmatter, DB status, and body content.
    for old_uri in [old_sequential, old_workflow] {
        assert_eq!(
            read_frontmatter_status(&env.cfg, old_uri),
            "abandoned",
            "frontmatter status should be abandoned for {old_uri}"
        );
        assert_eq!(
            read_db_status(&env.cfg, old_uri),
            "abandoned",
            "db status should be abandoned for {old_uri}"
        );
        let body = read_body(&env.cfg, old_uri);
        assert!(
            body.contains("## Session Abandoned"),
            "file body should contain abandon notice for {old_uri}, body: {body}"
        );
        let db_content = read_db_content(&env.cfg, old_uri);
        assert!(
            db_content.contains("## Session Abandoned"),
            "db content should contain abandon notice for {old_uri}"
        );
    }

    // Explicitly verify the recent session remains untouched in all layers.
    assert_eq!(
        read_frontmatter_status(&env.cfg, recent_active),
        "active",
        "recent session frontmatter should still be active"
    );
    assert_eq!(
        read_db_status(&env.cfg, recent_active),
        "active",
        "recent session db status should still be active"
    );
    let recent_body = read_body(&env.cfg, recent_active);
    assert!(
        !recent_body.contains("## Session Abandoned"),
        "recent session should not receive abandon notice, body: {recent_body}"
    );
}

// T-4.5b: RTM FR-4.2
#[test]
fn empty_database_returns_zero_abandoned_sessions() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 0);
    // Structural invariant: cleanup only SELECTs and UPDATEs — it should never INSERT rows.
    // This assertion documents intent rather than guarding against a plausible regression.
    assert_eq!(file_content_row_count(&env.cfg), 0);
}

#[test]
fn pipeline_owned_workflow_sessions_are_not_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let pipeline_uri = "memory://thinking/workflow/2026/03/09/workflow-pipeline-node";
    let standalone_uri = "memory://thinking/workflow/2026/03/09/workflow-standalone";

    // Pipeline-owned session: active for 60 min, but has pipelineId — must be skipped.
    create_pipeline_session(
        &env.cfg,
        pipeline_uri,
        "active",
        60,
        "# Workflow Session\n\n## Step 1\nPipeline [[node]] work",
        "pipeline-1774546611412",
    );
    // Standalone session: same age, no pipelineId — should be abandoned normally.
    create_session(
        &env.cfg,
        standalone_uri,
        "active",
        60,
        "# Workflow Session\n\n## Step 1\nStandalone [[workflow]]",
    );

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 1, "only the standalone session should be abandoned");
    assert_eq!(
        read_frontmatter_status(&env.cfg, pipeline_uri),
        "active",
        "pipeline-owned session frontmatter must remain active"
    );
    assert_eq!(
        read_db_status(&env.cfg, pipeline_uri),
        "active",
        "pipeline-owned session db status must remain active"
    );
    let pipeline_body = read_body(&env.cfg, pipeline_uri);
    assert!(
        !pipeline_body.contains("## Session Abandoned"),
        "pipeline-owned session must not receive abandon notice"
    );
    assert_eq!(
        read_frontmatter_status(&env.cfg, standalone_uri),
        "abandoned",
        "standalone session should be abandoned"
    );
}
