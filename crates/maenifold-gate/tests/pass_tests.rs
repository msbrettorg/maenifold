use std::{fs, path::Path};

use maenifold_gate::{
    append::{self, AppendArgs},
    db::{GateDb, TaskUpsert},
    pass::{self, PassArgs},
};
use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

fn setup() -> (TempDir, std::path::PathBuf, String, String) {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("todo")).unwrap();
    fs::create_dir_all(dir.path().join("done")).unwrap();
    let file = "T-55.4-gate-pass-command.md".to_string();
    let id = "T-55.4".to_string();
    fs::write(
        dir.path().join("todo").join(&file),
        "---\npipeline: sdd\n---\n\n# T-55.4 — Pass\n\n**Requirement**: FR-42\n",
    )
    .unwrap();
    let db_path = dir.path().join("gate.db");
    let task = TaskUpsert {
        id: &id,
        file: &file,
        title: Some("Pass"),
        requirement: "FR-42",
        status: "pending",
        location: "todo",
        attempts: 0,
    };
    GateDb::open(&db_path).unwrap().upsert_task(&task).unwrap();
    (dir, db_path, id, file)
}

fn run_pass(root: &Path, db_path: &Path, id: &str, notes: Option<&str>) {
    pass::run(&PassArgs {
        id: id.into(),
        notes: notes.map(str::to_owned),
        db_path: db_path.into(),
        root: root.into(),
    })
    .unwrap();
}

#[test]
fn pass_advances_pending_to_first_stage() {
    let (dir, db_path, id, file) = setup();
    run_pass(dir.path(), &db_path, &id, None);
    let task = GateDb::open(&db_path)
        .unwrap()
        .get_task(&file)
        .unwrap()
        .unwrap();
    assert_eq!(task.status, "swe");
    assert_eq!(task.location, "todo");
}

#[test]
fn pass_advances_through_full_pipeline() {
    let (dir, db_path, id, file) = setup();

    run_pass(dir.path(), &db_path, &id, Some("to swe"));
    run_pass(dir.path(), &db_path, &id, Some("to red"));

    append::run(&AppendArgs {
        id: id.clone(),
        agent: "red".into(),
        notes: "**Verdict**: PASS\n\nNo issues.".into(),
        db_path: db_path.clone().into(),
        root: dir.path().into(),
    })
    .unwrap();
    run_pass(dir.path(), &db_path, &id, Some("to blue"));

    append::run(&AppendArgs {
        id: id.clone(),
        agent: "blue".into(),
        notes: "**Verdict**: PASS\n\nAll tests pass.".into(),
        db_path: db_path.clone().into(),
        root: dir.path().into(),
    })
    .unwrap();

    run_pass(dir.path(), &db_path, &id, Some("to done"));

    let task = GateDb::open(&db_path)
        .unwrap()
        .get_task(&file)
        .unwrap()
        .unwrap();
    assert_eq!(task.status, "done");
    assert_eq!(task.location, "done");
    assert!(!dir.path().join("todo").join(&file).exists());
    assert!(dir.path().join("done").join(&file).exists());
    assert_eq!(
        Connection::open(&db_path)
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM work_log WHERE file = ?1",
                [&file],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
        4
    );
}

#[test]
fn pass_unknown_id_errors() {
    let (dir, db_path, ..) = setup();
    let err = pass::run(&PassArgs {
        id: "T-0".into(),
        notes: None,
        db_path,
        root: dir.path().into(),
    })
    .unwrap_err();
    assert!(err.to_string().contains("unknown task ID: T-0"));
}

#[test]
fn pass_appends_orchestrator_section() {
    let (dir, db_path, id, file) = setup();
    run_pass(dir.path(), &db_path, &id, Some("Ship it"));
    let content = fs::read_to_string(dir.path().join("todo").join(file)).unwrap();
    assert!(content.contains("## Orchestrator — PASS"));
    assert!(content.contains("Ship it"));
}

#[test]
fn pass_requires_pipeline_frontmatter() {
    let (dir, db_path, id, file) = setup();
    fs::write(
        dir.path().join("todo").join(&file),
        "# T-55.4 — Pass\n\n**Requirement**: FR-42\n",
    )
    .unwrap();

    let err = pass::run(&PassArgs {
        id,
        notes: None,
        db_path,
        root: dir.path().into(),
    })
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("task 'T-55.4' has no 'pipeline' field in frontmatter")
    );
}
