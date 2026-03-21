use std::fs;
use std::path::{Path, PathBuf};

use maenifold_gate::{
    AdvanceArgs, StageVerdict, StatusArgs, SyncArgs, TaskLocation, TaskStage, TaskStatus, advance,
    db::GateDb, parser, status, sync,
};
use tempfile::{TempDir, tempdir};

fn setup_root() -> TempDir {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("todo")).unwrap();
    fs::create_dir_all(dir.path().join("done")).unwrap();
    dir
}

fn task_file_name(n: u32) -> String {
    format!("T-99.{n}-test-task-{n}.md")
}

fn write_task(root: &Path, location: TaskLocation, file: &str, content: &str) -> PathBuf {
    let path = root.join(location.as_str()).join(file);
    fs::write(&path, content).unwrap();
    path
}

fn base_task(n: u32) -> String {
    format!(
        "# T-99.{n} — Test Task {n}\n\n**Requirement**: FR-41\n\n## Acceptance Criteria\n\n1. Test criterion\n"
    )
}

fn with_swe(mut content: String) -> String {
    content.push_str("\n## SWE Remediation\n\nImplementation done.\n");
    content
}

fn with_red_pass(mut content: String) -> String {
    content.push_str("\n## Red-team Audit\n\n**Verdict**: PASS\n\nNo issues found.\n");
    content
}

fn with_blue_pass(mut content: String) -> String {
    content.push_str("\n## Blue-team Verification\n\n**Verdict**: PASS\n\nAll tests pass.\n");
    content
}

fn rebuild_cache(root: &Path) -> PathBuf {
    let db_path = root.join("pipeline.db");
    let args = SyncArgs {
        db: Some(db_path.clone()),
        root: root.to_path_buf(),
        rebuild: true,
    };
    sync::run_sync(&args, root).unwrap();
    db_path
}

fn read_row(db_path: &Path, file: &str) -> maenifold_gate::db::TaskRow {
    GateDb::open(db_path)
        .unwrap()
        .get_task(file)
        .unwrap()
        .unwrap()
}

fn advance_args(root: &Path, db: Option<&Path>, file: &str, stage: TaskStage) -> AdvanceArgs {
    AdvanceArgs {
        task: PathBuf::from(file),
        stage,
        verdict: None,
        notes: None,
        model: None,
        root: Some(root.to_path_buf()),
        db: db.map(Path::to_path_buf),
    }
}

#[test]
fn test_sync_rebuild() {
    let root = setup_root();
    let pending_file = task_file_name(1);
    let red_file = task_file_name(2);
    let done_file = task_file_name(3);

    write_task(
        root.path(),
        TaskLocation::Todo,
        &pending_file,
        &base_task(1),
    );
    write_task(
        root.path(),
        TaskLocation::Todo,
        &red_file,
        &with_swe(base_task(2)),
    );
    write_task(
        root.path(),
        TaskLocation::Done,
        &done_file,
        &with_blue_pass(with_red_pass(with_swe(base_task(3)))),
    );

    let db_path = rebuild_cache(root.path());

    let pending = read_row(&db_path, &pending_file);
    assert_eq!(pending.title.as_deref(), Some("Test Task 1"));
    assert_eq!(pending.requirement, "FR-41");
    assert_eq!(pending.status, "pending");
    assert_eq!(pending.location, "todo");
    assert_eq!(pending.attempts, 0);

    let red = read_row(&db_path, &red_file);
    assert_eq!(red.title.as_deref(), Some("Test Task 2"));
    assert_eq!(red.requirement, "FR-41");
    assert_eq!(red.status, "red");
    assert_eq!(red.location, "todo");
    assert_eq!(red.attempts, 0);

    let done = read_row(&db_path, &done_file);
    assert_eq!(done.title.as_deref(), Some("Test Task 3"));
    assert_eq!(done.requirement, "FR-41");
    assert_eq!(done.status, "done");
    assert_eq!(done.location, "done");
    assert_eq!(done.attempts, 0);
}

#[test]
fn test_sync_reconcile_detects_drift() {
    let root = setup_root();
    let pending_file = task_file_name(1);
    let blue_file = task_file_name(2);
    let done_file = task_file_name(3);

    write_task(
        root.path(),
        TaskLocation::Todo,
        &pending_file,
        &base_task(1),
    );
    write_task(
        root.path(),
        TaskLocation::Todo,
        &blue_file,
        &with_red_pass(with_swe(base_task(2))),
    );
    write_task(
        root.path(),
        TaskLocation::Done,
        &done_file,
        &with_blue_pass(with_red_pass(with_swe(base_task(3)))),
    );

    let db_path = rebuild_cache(root.path());
    let db = GateDb::open(&db_path).unwrap();
    db.upsert_task(
        &blue_file,
        Some("Corrupted"),
        "FR-99",
        "pending",
        "done",
        7,
    )
    .unwrap();
    drop(db);

    let args = SyncArgs {
        db: Some(db_path.clone()),
        root: root.path().to_path_buf(),
        rebuild: false,
    };
    sync::run_sync(&args, root.path()).unwrap();

    let repaired = read_row(&db_path, &blue_file);
    assert_eq!(repaired.title.as_deref(), Some("Test Task 2"));
    assert_eq!(repaired.requirement, "FR-41");
    assert_eq!(repaired.status, "blue");
    assert_eq!(repaired.location, "todo");
    assert_eq!(repaired.attempts, 0);
}

#[test]
fn test_status_single_file() {
    let root = setup_root();
    let file = task_file_name(1);
    let path = write_task(
        root.path(),
        TaskLocation::Todo,
        &file,
        &with_red_pass(with_swe(base_task(1))),
    );
    let state = parser::parse_task_file(&path).unwrap();
    assert_eq!(state.status, TaskStatus::Blue);

    let db_path = rebuild_cache(root.path());
    let args = StatusArgs {
        db: Some(db_path),
        root: Some(root.path().to_path_buf()),
        json: false,
        task: PathBuf::from(&file),
    };

    status::run_status(&args, root.path()).unwrap();
}

#[test]
fn test_status_all_files_summary() {
    let root = setup_root();
    write_task(
        root.path(),
        TaskLocation::Todo,
        &task_file_name(1),
        &base_task(1),
    );
    write_task(
        root.path(),
        TaskLocation::Todo,
        &task_file_name(2),
        &with_swe(base_task(2)),
    );
    write_task(
        root.path(),
        TaskLocation::Done,
        &task_file_name(3),
        &with_blue_pass(with_red_pass(with_swe(base_task(3)))),
    );

    let db_path = rebuild_cache(root.path());
    let args = StatusArgs {
        db: Some(db_path),
        root: Some(root.path().to_path_buf()),
        json: false,
        task: PathBuf::new(),
    };

    status::run_status(&args, root.path()).unwrap();
}

#[test]
fn test_advance_then_sync_roundtrip() {
    let root = setup_root();
    let file = task_file_name(1);
    write_task(
        root.path(),
        TaskLocation::Todo,
        &file,
        &base_task(1),
    );

    let db_path = root.path().join("pipeline.db");
    GateDb::open(&db_path).unwrap();

    let mut args = advance_args(root.path(), Some(&db_path), &file, TaskStage::Swe);
    args.notes = Some("Implementation done.".into());
    advance::run_advance(&args, root.path()).unwrap();

    args.stage = TaskStage::Red;
    args.verdict = Some(StageVerdict::Pass);
    args.notes = Some("No issues found.".into());
    advance::run_advance(&args, root.path()).unwrap();

    args.stage = TaskStage::Blue;
    args.verdict = Some(StageVerdict::Pass);
    args.notes = Some("All tests pass.".into());
    advance::run_advance(&args, root.path()).unwrap();

    let done_path = root.path().join("done").join(&file);
    assert!(!root.path().join("todo").join(&file).exists());
    assert!(done_path.exists());
    let final_state = parser::parse_task_file(&done_path).unwrap();
    assert_eq!(final_state.status, TaskStatus::Done);
    assert_eq!(final_state.location, TaskLocation::Done);

    let sync_args = SyncArgs {
        db: Some(db_path.clone()),
        root: root.path().to_path_buf(),
        rebuild: false,
    };
    sync::run_sync(&sync_args, root.path()).unwrap();

    let row = read_row(&db_path, &file);
    assert_eq!(row.requirement, "FR-41");
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert_eq!(row.attempts, 0);
}

#[test]
fn test_advance_errors_on_wrong_transition() {
    let root = setup_root();
    let file = task_file_name(1);
    write_task(
        root.path(),
        TaskLocation::Todo,
        &file,
        &base_task(1),
    );

    let mut args = advance_args(root.path(), None, &file, TaskStage::Blue);
    args.verdict = Some(StageVerdict::Pass);
    let error = advance::run_advance(&args, root.path()).unwrap_err();

    assert!(error.to_string().contains("Cannot advance"));
    assert!(error.to_string().contains("pending"));
}
