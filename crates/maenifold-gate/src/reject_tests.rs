use super::{RejectArgs, run};
use crate::db::GateDb;
use rusqlite::Connection;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

struct TestEnv {
    _dir: TempDir,
    root: PathBuf,
    db: PathBuf,
    file: String,
}

fn setup(status: &str, pipeline: &str) -> TestEnv {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("todo")).unwrap();
    fs::create_dir_all(root.join("done")).unwrap();
    let file = format!("T-55.5-{status}.md");
    fs::write(
        root.join("todo").join(&file),
        format!("---\nstatus: {status}\npipeline: {pipeline}\n---\n\n# Task\n"),
    )
    .unwrap();
    let db = root.join("gate.db");
    GateDb::open(&db)
        .unwrap()
        .upsert_task(&crate::db::TaskUpsert {
            id: "T-55.5",
            file: &file,
            title: Some("Task"),
            requirement: "FR-42",
            status,
            location: "todo",
            attempts: 0,
        })
        .unwrap();
    TestEnv {
        _dir: dir,
        root,
        db,
        file,
    }
}

#[test]
fn reject_at_red_returns_to_pending() {
    let env = setup("red", "sdd");
    run(&RejectArgs {
        id: "T-55.5".into(),
        notes: Some("retry".into()),
        db_path: env.db.clone(),
        root: env.root.clone(),
    })
    .unwrap();
    let task = GateDb::open(&env.db)
        .unwrap()
        .get_task_by_id("T-55.5")
        .unwrap()
        .unwrap();
    assert_eq!((task.status, task.attempts), ("pending".into(), 1));
    assert!(
        fs::read_to_string(env.root.join("todo").join(&env.file))
            .unwrap()
            .contains("## Orchestrator — REJECT\n\nretry")
    );
    assert_eq!(
        Connection::open(&env.db)
            .unwrap()
            .query_row(
                "SELECT stage || ':' || verdict FROM work_log ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "orchestrator:REJECT"
    );
}

#[test]
fn reject_at_implement_returns_to_pending() {
    let env = setup("implement", "tdd");
    run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        db_path: env.db.clone(),
        root: env.root.clone(),
    })
    .unwrap();
    let task = GateDb::open(&env.db)
        .unwrap()
        .get_task_by_id("T-55.5")
        .unwrap()
        .unwrap();
    assert_eq!((task.status, task.attempts), ("pending".into(), 1));
}

#[test]
fn reject_at_swe_errors() {
    let env = setup("swe", "sdd");
    let err = run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        db_path: env.db,
        root: env.root,
    })
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("stage 'swe' has no reject transition")
    );
}

#[test]
fn reject_at_pending_errors() {
    let env = setup("pending", "sdd");
    let err = run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        db_path: env.db,
        root: env.root,
    })
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("task status 'pending' is not a valid pipeline stage")
    );
}

#[test]
fn reject_unknown_id_errors() {
    let env = setup("red", "sdd");
    let err = run(&RejectArgs {
        id: "T-404".into(),
        notes: None,
        db_path: env.db,
        root: env.root,
    })
    .unwrap_err();
    assert!(err.to_string().contains("unknown task ID: T-404"));
}

#[test]
fn reject_requires_pipeline_frontmatter() {
    let env = setup("red", "sdd");
    fs::write(env.root.join("todo").join(&env.file), "# Task\n").unwrap();
    let err = run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        db_path: env.db,
        root: env.root,
    })
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("task 'T-55.5' has no 'pipeline' field in frontmatter")
    );
}
