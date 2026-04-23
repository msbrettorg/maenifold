use std::fs;

use rusqlite::params;
use serde_json::Value;
use tempfile::tempdir;

use super::{GateCacheDb, ensure_schema};
use crate::error::GateError;
use crate::task_repository::TaskPathSet;

#[test]
fn rebuild_caches_linked_status_leniently() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");

    let linked_path = root.join("todo").join("T-90.27.md");
    fs::write(
        &linked_path,
        "---\ntitle: Linked task\npipeline: sdd\nsessionId: session-1775916753112-57359\n---\n# Linked task\n",
    )
    .expect("write linked task");

    let malformed_path = root.join("todo").join("T-90.28.md");
    fs::write(
        &malformed_path,
        "---\ntitle: Malformed linkage\npipeline: sdd\nsessionId:\n  - malformed\n---\n# Malformed linkage\n",
    )
    .expect("write malformed task");

    GateCacheDb::rebuild(
        root,
        &[
            TaskPathSet {
                task_id: "T-90.27".to_string(),
                path: linked_path,
                location: "todo".to_string(),
            },
            TaskPathSet {
                task_id: "T-90.28".to_string(),
                path: malformed_path,
                location: "todo".to_string(),
            },
        ],
    )
    .expect("rebuild cache db");

    let rows = GateCacheDb::open(root)
        .expect("open db")
        .list_task_status(root, None)
        .expect("load cached status");
    assert_eq!(rows[0]["linked"], true);
    assert_eq!(rows[1]["linked"], false);
}

#[cfg(unix)]
#[test]
fn rebuild_reports_symlinked_task_without_parsing_target() {
    use std::os::unix::fs as unix_fs;

    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");

    let target = root.join("todo").join("status-target.md");
    fs::write(
        &target,
        "---\nid: T-999.9\ntitle: Status target\npipeline: sdd\n---\n# Status target\n",
    )
    .expect("write status target");
    let task_path = root.join("todo").join("T-90.29.md");
    unix_fs::symlink(&target, &task_path).expect("create symlinked task");

    let error = GateCacheDb::rebuild(
        root,
        &[TaskPathSet {
            task_id: "T-90.29".to_string(),
            path: task_path,
            location: "todo".to_string(),
        }],
    )
    .expect_err("symlinked task should be surfaced");
    assert!(error.to_string().contains("through a symlink"));

    let rows = GateCacheDb::open(root)
        .expect("open db")
        .list_task_status(root, None)
        .expect("load cached status");
    assert_eq!(rows[0]["taskId"], "T-90.29");
    assert_eq!(rows[0]["canonicalTaskId"], Value::Null);
    assert_eq!(rows[0]["state"], "error");
    assert!(
        rows[0]["issue"]
            .as_str()
            .expect("issue string")
            .contains("through a symlink")
    );
}

#[test]
fn list_task_status_rejects_duplicate_cached_task_ids() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path();
    let db = GateCacheDb::open(root).expect("open cache db");
    let mut connection = db.connection().expect("open sqlite connection");
    let transaction = connection.transaction().expect("create transaction");
    ensure_schema(&transaction).expect("ensure schema");
    transaction
        .execute(
            "INSERT INTO tasks (
                file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, issue
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                "todo/T-90.25-a.md",
                "T-90.25",
                "T-90.25",
                "First duplicate",
                "sdd",
                "pending",
                "todo",
                0,
                "[]",
                ""
            ],
        )
        .expect("insert first task row");
    transaction
        .execute(
            "INSERT INTO tasks (
                file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, issue
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                "done/T-90.25-b.md",
                "T-90.25",
                "T-90.25",
                "Second duplicate",
                "sdd",
                "done",
                "done",
                0,
                "[]",
                ""
            ],
        )
        .expect("insert second task row");
    transaction.commit().expect("commit rows");

    let error = db
        .list_task_status(root, None)
        .expect_err("duplicate cached ids should fail");
    match error {
        GateError::DuplicateCanonicalTaskId {
            task_id,
            first_path,
            second_path,
        } => {
            assert_eq!(task_id, "T-90.25");
            assert!(first_path.ends_with("todo/T-90.25-a.md"));
            assert!(second_path.ends_with("done/T-90.25-b.md"));
        }
        other => panic!("expected duplicate canonical task id error, got {other}"),
    }
}

#[cfg(unix)]
#[test]
fn cache_db_connection_on_readonly_path_includes_path_in_error() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("create temp root");
    let root = temp.path().join("readonly_project");
    fs::create_dir_all(&root).expect("create readonly dir");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o444)).expect("make dir read-only");

    let db = GateCacheDb::open(&root).expect("open cache db");
    let err = db.connection().expect_err("read-only root should fail");
    let msg = err.to_string();
    assert!(
        msg.contains(".gate"),
        "error should mention .gate directory: {msg}"
    );
    assert!(
        msg.contains("writable") || msg.contains("--root"),
        "error should contain remediation hint: {msg}"
    );

    // Cleanup: restore permissions so temp dir can be removed
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("restore permissions");
}
