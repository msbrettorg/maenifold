// T-W1-7: RTM FR-14.6
#[path = "workflow_supervisor/support.rs"]
mod support;
#[path = "workflow_supervisor/sync_support.rs"]
mod sync_support;

use chrono::{Duration, Utc};
use std::fs;

use support::*;
use sync_support::{
    create_normal_memory_file, create_workflow_session_fixture, file_mtime_rfc3339,
    set_file_mtime_utc,
};

// T-W1-7: RTM FR-14.6
#[test]
fn sync_stale_session_marked_abandoned_even_when_mtime_guard_would_skip_read() {
    let _guard = TEST_MUTEX.lock();
    let _env_guard = SessionCleanupEnvGuard::set_abandonment_minutes("60");

    for session_type in ["workflow", "sequential"] {
        let env = isolated_env();
        let session_id = create_workflow_session_fixture(&env.cfg, session_type, "active");
        let file_uri = session_uri(session_type, &session_id);
        let path = path_for_uri(&env.cfg, &file_uri);

        invoke_sync(&env.cfg);

        // Keep the file well past the pinned 60-minute abandonment threshold so sync's
        // mtime guard would skip re-reading it unless cleanup still rewrites the session.
        let stale = Utc::now() - Duration::minutes(130);
        set_file_mtime_utc(&path, stale);
        let last_indexed = file_mtime_rfc3339(&path);
        update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, Some("active"));

        invoke_sync(&env.cfg);

        assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
        assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
    }
}

// T-W1-7: RTM FR-14.6
#[test]
fn sync_does_not_rewrite_normal_memory_file_solely_due_to_age_mtime_guard_remains() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let (path, file_uri) = create_normal_memory_file(
        &env.cfg,
        "notes/cleanup-001-1/note.md",
        "# Normal Note\n\nThis is a normal file, not a thinking session. [[Cleanup Note]]\n",
    );

    invoke_sync(&env.cfg);

    // Use a much larger offset than any abandonment threshold so any accidental rewrite of
    // a normal memory file is clearly caused by cleanup rather than threshold ambiguity.
    let stale = Utc::now() - Duration::days(7);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, None);

    let before_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let before_text = fs::read_to_string(&path).expect("read before");

    invoke_sync(&env.cfg);

    let after_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let after_text = fs::read_to_string(&path).expect("read after");
    assert_eq!(before_mtime, after_mtime);
    assert_eq!(before_text, after_text);
}
