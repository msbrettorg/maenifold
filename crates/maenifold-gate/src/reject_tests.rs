use super::{RejectArgs, run};
use crate::db::GateDb;
use rusqlite::Connection;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

struct TestEnv {
    _dir: TempDir,
    root: PathBuf,
    db: PathBuf,
    pipeline: PathBuf,
    file: String,
}

fn setup(status: &str) -> TestEnv {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("todo")).unwrap();
    fs::create_dir_all(root.join("done")).unwrap();
    let file = format!("T-55.5-{status}.md");
    fs::write(root.join("todo").join(&file), "# Task\n").unwrap();
    let pipeline = root.join("pipeline.json");
    fs::write(
        &pipeline,
        r#"{"name":"test","stages":[
{"name":"swe","verdicts":[],"transitions":{"pass":"red"}},
{"name":"red","verdicts":["pass","reject"],"transitions":{"pass":"blue","reject":"pending"}},
{"name":"blue","verdicts":["pass","reject"],"transitions":{"pass":"done","reject":"pending"}}]}"#,
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
        pipeline,
        file,
    }
}

#[test]
fn reject_at_red_returns_to_pending() {
    let env = setup("red");
    run(&RejectArgs {
        id: "T-55.5".into(),
        notes: Some("retry".into()),
        pipeline_path: Some(env.pipeline.clone()),
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
fn reject_at_blue_returns_to_pending() {
    let env = setup("blue");
    run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        pipeline_path: Some(env.pipeline.clone()),
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
    let env = setup("swe");
    let err = run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        pipeline_path: Some(env.pipeline),
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
    let env = setup("pending");
    let err = run(&RejectArgs {
        id: "T-55.5".into(),
        notes: None,
        pipeline_path: Some(env.pipeline),
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
    let env = setup("red");
    let err = run(&RejectArgs {
        id: "T-404".into(),
        notes: None,
        pipeline_path: Some(env.pipeline),
        db_path: env.db,
        root: env.root,
    })
    .unwrap_err();
    assert!(err.to_string().contains("unknown task ID: T-404"));
}
