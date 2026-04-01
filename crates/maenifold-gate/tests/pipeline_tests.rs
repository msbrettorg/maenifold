use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use maenifold_gate::{
    StatusArgs, SyncArgs,
    append::{self, AppendArgs},
    db::{GateDb, TaskRow},
    pass::{self, PassArgs},
    pipeline,
    reject::{self, RejectArgs},
    status, sync,
};
use tempfile::TempDir;

const SDD_PIPELINE: &str = "sdd";
const TDD_PIPELINE: &str = "tdd";
const LIGHTWEIGHT_PIPELINE: &str = "lightweight";

fn setup_test_env(pipeline_name: &str) -> (TempDir, PathBuf, PathBuf) {
    pipeline::load_embedded_pipeline(pipeline_name).unwrap();

    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(root.join("todo")).unwrap();
    fs::create_dir_all(root.join("done")).unwrap();

    let db_path = root.join("gate.db");
    (tmp, root, db_path)
}

fn create_task(root: &Path, filename: &str, pipeline_name: &str) {
    let id = filename
        .trim_end_matches(".md")
        .splitn(3, '-')
        .take(2)
        .collect::<Vec<_>>()
        .join("-");
    let content = format!(
        "---\nid: {id}\ntitle: Test Task\nstatus: pending\npipeline: {pipeline_name}\n---\n\n# {id} — Test Task\n\n**Requirement**: FR-42\n\n## Acceptance Criteria\n- test\n"
    );
    fs::write(root.join("todo").join(filename), content).unwrap();
}

fn create_done_task(root: &Path, filename: &str, pipeline_name: &str) {
    let id = filename
        .trim_end_matches(".md")
        .splitn(3, '-')
        .take(2)
        .collect::<Vec<_>>()
        .join("-");
    let content = format!(
        "---\nid: {id}\ntitle: Done Task\nstatus: done\npipeline: {pipeline_name}\n---\n\n# {id} — Done Task\n\n**Requirement**: FR-42\n\n## Acceptance Criteria\n- done\n"
    );
    fs::write(root.join("done").join(filename), content).unwrap();
}

fn sync_db(root: &Path, db_path: &Path) {
    sync::run_sync(
        &SyncArgs {
            db: Some(db_path.to_path_buf()),
            root: root.to_path_buf(),
            rebuild: true,
        },
        root,
    )
    .unwrap();
}

fn run_pass(root: &Path, db_path: &Path, id: &str) -> Result<()> {
    pass::run(&PassArgs {
        id: id.to_string(),
        notes: None,
        db_path: db_path.to_path_buf(),
        root: root.to_path_buf(),
    })
}

fn run_reject(root: &Path, db_path: &Path, id: &str) -> Result<()> {
    reject::run(&RejectArgs {
        id: id.to_string(),
        notes: None,
        db_path: db_path.to_path_buf(),
        root: root.to_path_buf(),
    })
}

fn run_append(root: &Path, db_path: &Path, id: &str, agent: &str, notes: &str) -> Result<()> {
    append::run(&AppendArgs {
        id: id.to_string(),
        notes: notes.to_string(),
        agent: agent.to_string(),
        db_path: db_path.to_path_buf(),
        root: root.to_path_buf(),
    })
}

fn append_verdict(root: &Path, db_path: &Path, id: &str, agent: &str, verdict: &str) {
    run_append(
        root,
        db_path,
        id,
        agent,
        &format!("- **Verdict**: {verdict}"),
    )
    .unwrap();
}

fn read_task_by_id(db_path: &Path, id: &str) -> TaskRow {
    GateDb::open(db_path)
        .unwrap()
        .get_task_by_id(id)
        .unwrap()
        .unwrap()
}

#[test]
fn full_tdd_pipeline_pass_through() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    let file = "T-55.7-full-tdd-flow.md";
    let id = "T-55.7";
    create_task(&root, file, TDD_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    run_pass(&root, &db_path, id).unwrap();
    append_verdict(&root, &db_path, id, "implement", "PASS");
    run_pass(&root, &db_path, id).unwrap();
    append_verdict(&root, &db_path, id, "verify", "PASS");
    run_pass(&root, &db_path, id).unwrap();

    let row = read_task_by_id(&db_path, id);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert!(!root.join("todo").join(file).exists());
    assert!(root.join("done").join(file).exists());
}

#[test]
fn sdd_pipeline_uses_red_then_blue() {
    let (_tmp, root, db_path) = setup_test_env(SDD_PIPELINE);
    let file = "T-55.7-sdd-flow.md";
    let id = "T-55.7";
    create_task(&root, file, SDD_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    assert_eq!(read_task_by_id(&db_path, id).status, "swe");

    run_pass(&root, &db_path, id).unwrap();
    assert_eq!(read_task_by_id(&db_path, id).status, "red");

    append_verdict(&root, &db_path, id, "red", "PASS");
    run_pass(&root, &db_path, id).unwrap();
    assert_eq!(read_task_by_id(&db_path, id).status, "blue");
}

#[test]
fn reject_at_red_returns_to_pending() {
    let (_tmp, root, db_path) = setup_test_env(SDD_PIPELINE);
    let file = "T-55.7-reject-red.md";
    let id = "T-55.7";
    create_task(&root, file, SDD_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    run_pass(&root, &db_path, id).unwrap();
    run_reject(&root, &db_path, id).unwrap();

    let row = read_task_by_id(&db_path, id);
    assert_eq!(row.status, "pending");
    assert_eq!(row.attempts, 1);
    assert_eq!(row.location, "todo");
    assert!(root.join("todo").join(file).exists());
    assert!(!root.join("done").join(file).exists());
}

#[test]
fn append_logs_without_state_change() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    let file = "T-55.7-append.md";
    let id = "T-55.7";
    create_task(&root, file, TDD_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    run_append(
        &root,
        &db_path,
        id,
        "swe",
        "Implemented the requested changes",
    )
    .unwrap();

    let row = read_task_by_id(&db_path, id);
    assert_eq!(row.status, "test");
    assert_eq!(row.location, "todo");

    let content = fs::read_to_string(root.join("todo").join(file)).unwrap();
    assert!(content.contains("## Swe Notes"));
    assert!(content.contains("Implemented the requested changes"));
}

#[test]
fn lightweight_pipeline_skips_red() {
    let (_tmp, root, db_path) = setup_test_env(LIGHTWEIGHT_PIPELINE);
    let file = "T-55.7-lightweight.md";
    let id = "T-55.7";
    create_task(&root, file, LIGHTWEIGHT_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    assert_eq!(read_task_by_id(&db_path, id).status, "swe");

    run_pass(&root, &db_path, id).unwrap();
    assert_eq!(read_task_by_id(&db_path, id).status, "blue");

    append_verdict(&root, &db_path, id, "blue", "PASS");
    run_pass(&root, &db_path, id).unwrap();
    let row = read_task_by_id(&db_path, id);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert!(root.join("done").join(file).exists());
}

#[test]
fn unknown_id_errors() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    sync_db(&root, &db_path);

    let pass_err = run_pass(&root, &db_path, "T-99.99").unwrap_err();
    assert!(pass_err.to_string().contains("unknown task ID"));

    let reject_err = run_reject(&root, &db_path, "T-99.99").unwrap_err();
    assert!(reject_err.to_string().contains("unknown task ID"));

    let append_err = run_append(&root, &db_path, "T-99.99", "swe", "notes").unwrap_err();
    assert!(append_err.to_string().contains("unknown task ID"));
}

#[test]
fn reject_at_test_errors() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    let file = "T-55.7-reject-test.md";
    let id = "T-55.7";
    create_task(&root, file, TDD_PIPELINE);
    sync_db(&root, &db_path);

    run_pass(&root, &db_path, id).unwrap();
    let err = run_reject(&root, &db_path, id).unwrap_err();

    assert!(
        err.to_string()
            .contains("stage 'test' has no reject transition"),
        "unexpected error: {err}"
    );
}

#[test]
fn pass_done_task_errors() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    let file = "T-55.7-done-task.md";
    let id = "T-55.7";
    create_done_task(&root, file, TDD_PIPELINE);
    sync_db(&root, &db_path);

    let err = run_pass(&root, &db_path, id).unwrap_err();
    assert!(err.to_string().contains("is not in todo/"));
}

#[test]
fn status_unknown_id_errors() {
    let (_tmp, root, db_path) = setup_test_env(TDD_PIPELINE);
    let err = status::run_status(
        &StatusArgs {
            db: Some(db_path),
            root,
            json: false,
            id: Some("T-99.99".to_string()),
        },
        Path::new("."),
    )
    .unwrap_err();

    assert!(err.to_string().contains("unknown task ID"));
}
