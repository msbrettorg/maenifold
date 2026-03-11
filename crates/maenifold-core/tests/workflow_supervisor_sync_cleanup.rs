// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;
#[path = "workflow_supervisor/sync_support.rs"]
mod sync_support;

use chrono::{Duration, Utc};
use rusqlite::{Connection, params};
use std::fs;

use support::*;
use sync_support::{
    create_normal_memory_file, create_workflow_session_fixture, file_mtime_rfc3339,
    set_file_mtime_utc,
};

// T-W1-7: RTM FR-14.6
#[test]
fn sync_stale_workflow_session_marked_abandoned_even_when_mtime_guard_would_skip_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let session_id = create_workflow_session_fixture(&env.cfg, "workflow", "active");
    let file_uri = workflow_uri(&session_id);
    let path = path_for_uri(&env.cfg, &file_uri);

    invoke_sync(&env.cfg);

    let stale = Utc::now() - Duration::minutes(130);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, Some("active"));

    let last_indexed_db = Connection::open(&env.cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            params![&file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("last indexed");
    assert_eq!(last_indexed_db, last_indexed);

    invoke_sync(&env.cfg);

    assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
    assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
}

// T-W1-7: RTM FR-14.6
#[test]
fn sync_stale_sequential_session_marked_abandoned_even_when_mtime_guard_would_skip_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let session_id = create_workflow_session_fixture(&env.cfg, "sequential", "active");
    let file_uri = sequential_uri(&session_id);
    let path = path_for_uri(&env.cfg, &file_uri);

    invoke_sync(&env.cfg);

    let stale = Utc::now() - Duration::minutes(130);
    set_file_mtime_utc(&path, stale);
    let last_indexed = file_mtime_rfc3339(&path);
    update_last_indexed_and_status(&env.cfg, &file_uri, &last_indexed, Some("active"));

    let last_indexed_db = Connection::open(&env.cfg.database_path)
        .expect("open db")
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            params![&file_uri],
            |row| row.get::<_, String>(0),
        )
        .expect("last indexed");
    assert_eq!(last_indexed_db, last_indexed);

    invoke_sync(&env.cfg);

    assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
    assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
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
