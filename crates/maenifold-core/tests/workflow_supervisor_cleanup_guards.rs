// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;

use maenifold_core::sessions;
use support::*;

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_already_abandoned_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/already-abandoned-test";
    create_session(
        &env.cfg,
        file_uri,
        "abandoned",
        130,
        "# Abandoned Session\n\nContent [[Session Cleanup]]",
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}
