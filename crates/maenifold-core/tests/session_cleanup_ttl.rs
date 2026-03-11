#[path = "session/session_cleanup_support.rs"]
mod session_cleanup_support;

use session_cleanup_support::{
    SessionCleanupEnvGuard, TEST_MUTEX, cleanup_abandoned_sessions, create_session, isolated_env,
    read_body, read_db_content, read_db_status, read_frontmatter_status,
};

// T-4.5b: RTM FR-4.2
#[test]
fn active_session_within_default_ttl_is_not_marked_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/2026/03/09/session-within-ttl";
    create_session(
        &env.cfg,
        file_uri,
        "active",
        30,
        "# Session\n\n## Thought 1\nStill active [[Session Cleanup]]",
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

// T-4.5b: RTM FR-4.2
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
    let body = read_body(&env.cfg, file_uri);
    assert!(
        body.contains("## Session Abandoned"),
        "missing notice: {body}"
    );
}
