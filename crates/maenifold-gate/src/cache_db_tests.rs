use std::fs;

use tempfile::tempdir;

use super::GateCacheDb;
use crate::task_repository::TaskPathSet;

#[test]
fn gate_cache_connections_enable_wal_mode() {
    let temp = tempfile::tempdir().expect("create temp root");
    let db = GateCacheDb::open(temp.path()).expect("open cache db");
    let connection = db.connection().expect("open sqlite connection");
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .expect("read journal mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
}

#[test]
fn rebuild_preserves_work_log_history() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    let db = GateCacheDb::open(root).expect("open cache db");
    GateCacheDb::rebuild(root, &[]).expect("create cache db schema");
    let connection = db.connection().expect("open sqlite connection");
    connection
        .execute(
            "INSERT INTO work_log (file, stage, agent, notes, created_at)
             VALUES ('sync', 'sync', 'gate', 'before rebuild', datetime('now'))",
            [],
        )
        .expect("insert work log entry");

    GateCacheDb::rebuild(root, &[]).expect("rebuild cache db");

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM work_log", [], |row| row.get(0))
        .expect("count work log rows");
    assert_eq!(count, 1);
}

#[test]
fn incremental_sync_preserves_task_row_identity_while_rebuild_replaces_it() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    let task_path = root.join("todo").join("T-90.15.md");
    fs::write(
        &task_path,
        "# T-90.15 — Cache refresh\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. sync refreshes read model\n",
    )
    .expect("write task");

    let task = TaskPathSet {
        task_id: "T-90.15".to_string(),
        path: task_path.clone(),
        location: "todo".to_string(),
    };

    GateCacheDb::rebuild(root, &[task.clone()]).expect("seed cache");
    let db = GateCacheDb::open(root).expect("open cache db");
    let connection = db.connection().expect("open sqlite connection");
    let (original_title, original_created_at): (String, String) = connection
        .query_row(
            "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read seeded row");

    connection
        .execute(
            "UPDATE tasks SET title = 'stale title', created_at = '2000-01-01 00:00:00'
             WHERE file = 'todo/T-90.15.md'",
            [],
        )
        .expect("mutate cached row");

    GateCacheDb::sync(root, &[task.clone()]).expect("incremental sync");
    let incremental = connection
        .query_row(
            "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read synced row");
    assert_eq!(incremental.0, original_title);
    assert_eq!(incremental.1, "2000-01-01 00:00:00");

    GateCacheDb::rebuild(root, &[task]).expect("rebuild cache");
    let rebuilt = connection
        .query_row(
            "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read rebuilt row");
    assert_eq!(rebuilt.0, original_title);
    assert_ne!(rebuilt.1, "2000-01-01 00:00:00");
    assert!(rebuilt.1 >= original_created_at);
}

#[test]
fn rebuild_reports_task_issue_details_after_caching_rows() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    let task_path = root.join("todo").join("T-90.26.md");
    fs::write(
        &task_path,
        "---\nid: [\npipeline: sdd\n---\n# Broken frontmatter\n",
    )
    .expect("write malformed task");

    let error = GateCacheDb::rebuild(
        root,
        &[TaskPathSet {
            task_id: "T-90.26".to_string(),
            path: task_path,
            location: "todo".to_string(),
        }],
    )
    .expect_err("malformed task should be surfaced");
    let message = error.to_string();
    assert!(message.contains("gate sync found task issues"));
    assert!(message.contains("T-90.26.md"));
    assert!(message.contains("invalid task frontmatter"));

    let rows = GateCacheDb::open(root)
        .expect("open db")
        .list_task_status(root, None)
        .expect("load cached status");
    assert_eq!(rows[0]["state"], "error");
    assert!(
        rows[0]["issue"]
            .as_str()
            .expect("issue string")
            .contains("invalid task frontmatter")
    );
    assert_eq!(rows[0]["linked"], false);
}
