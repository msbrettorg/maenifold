#[path = "session/session_cleanup_support.rs"]
mod session_cleanup_support;

use chrono::{Duration, Timelike, Utc};
use session_cleanup_support::{
    SessionCleanupEnvGuard, TEST_MUTEX, align_to_second_boundary, cleanup_abandoned_sessions,
    create_session, isolated_env, read_body, read_db_content, read_db_status,
    read_frontmatter_status, seed_file_content, write_session_markdown,
};

// T-4.5b: RTM FR-4.2 — boundary: session at exact TTL edge escapes cleanup (strict `<`)
#[test]
fn active_session_at_exact_ttl_boundary_is_not_marked_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-at-ttl-boundary";
    let body = "# Session\n\n## Thought 1\nStill active [[Session Cleanup]]";
    write_session_markdown(&env.cfg, file_uri, "active", body);

    // Align to a second boundary so T₀ (seed) and T₁ (cleanup's Utc::now()) share the
    // same truncated second — eliminates the latent timing fragility.
    align_to_second_boundary();
    let anchor = Utc::now().with_nanosecond(0).expect("truncate nanos");
    let exact_boundary = (anchor - Duration::minutes(30)).to_rfc3339();
    seed_file_content(&env.cfg, file_uri, "active", body, &exact_boundary);

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
    assert_eq!(read_db_status(&env.cfg, file_uri), "active");
    let read_body = read_body(&env.cfg, file_uri);
    assert!(
        !read_body.contains("## Session Abandoned"),
        "unexpected body: {read_body}"
    );
}

// T-4.5b: RTM FR-4.2 — interior: session well within TTL window is untouched
#[test]
fn active_session_well_within_ttl_is_not_marked_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-within-ttl";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        15,
        "# Session\n\n## Thought 1\nRecently active [[Session Cleanup]]",
    );

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

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

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

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

// T-4.5b: RTM FR-4.2 — env-var TTL override with full dual-storage assertions
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

    let abandoned = cleanup_abandoned_sessions(&env.cfg);

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

// T-4.5b: RTM FR-4.2 — invalid env var values silently fall back to 30-min default
#[test]
fn invalid_env_var_falls_back_to_default_ttl() {
    for invalid_value in &["abc", "0", "-1", ""] {
        let _guard = TEST_MUTEX.lock();
        let _env_guard = SessionCleanupEnvGuard::set_abandonment_minutes(invalid_value);
        let env = isolated_env();
        let file_uri = "memory://thinking/sequential/2026/03/09/session-invalid-env";
        create_session(
            &env.cfg,
            file_uri,
            "active",
            31,
            "# Session\n\n## Thought 1\nPast default TTL [[Session Cleanup]]",
        );

        let abandoned = cleanup_abandoned_sessions(&env.cfg);

        assert_eq!(
            abandoned, 1,
            "MAENIFOLD_SESSION_ABANDON_MINUTES={invalid_value:?} should fall back to 30-min default"
        );
        assert_eq!(
            read_frontmatter_status(&env.cfg, file_uri),
            "abandoned",
            "MAENIFOLD_SESSION_ABANDON_MINUTES={invalid_value:?}: frontmatter not updated"
        );
    }
}
