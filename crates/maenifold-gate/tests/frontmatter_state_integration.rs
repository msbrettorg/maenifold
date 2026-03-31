use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use maenifold_gate::{
    SyncArgs, TaskLocation,
    db::{GateDb, extract_task_id},
    pass::{self, PassArgs},
    sync,
};
use serde_json::Value;
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

fn write_pipeline(root: &Path, content: &str) {
    fs::write(root.join("pipeline.json"), content).unwrap();
}

fn base_task(n: u32) -> String {
    format!(
        "# T-99.{n} — Test Task {n}\n\n**Requirement**: FR-41\n\nAcceptance Criteria:\n\n1. Test criterion\n"
    )
}

fn with_swe(mut content: String) -> String {
    content.push_str("\n## SWE Remediation\n\nImplementation done.\n");
    content
}

fn with_red_pass(mut content: String) -> String {
    content.push_str("\n## Red Team Audit\n\n**Verdict**: PASS\n\nNo issues found.\n");
    content
}

fn with_blue_pass(mut content: String) -> String {
    content.push_str("\n## Blue Team Verification\n\n**Verdict**: PASS\n\nAll tests pass.\n");
    content
}

fn with_frontmatter(status: &str, attempts: u32, body: String) -> String {
    format!("---\nstatus: {status}\nattempts: {attempts}\n---\n\n{body}")
}

fn rebuild_cache(root: &Path) -> PathBuf {
    let db_path = root.join("gate.db");
    let args = SyncArgs {
        db: Some(db_path.clone()),
        root: root.to_path_buf(),
        rebuild: true,
        pipeline_path: None,
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

fn run_pass(root: &Path, db_path: &Path, id: &str, notes: Option<&str>) {
    pass::run(&PassArgs {
        id: id.into(),
        notes: notes.map(str::to_owned),
        pipeline_path: None,
        db_path: db_path.into(),
        root: root.into(),
    })
    .unwrap();
}

fn resync(root: &Path, db_path: &Path) {
    let sync_args = SyncArgs {
        db: Some(db_path.to_path_buf()),
        root: root.to_path_buf(),
        rebuild: false,
        pipeline_path: None,
    };
    sync::run_sync(&sync_args, root).unwrap();
}

fn gate_command() -> Command {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_gate") {
        Command::new(path)
    } else {
        cargo_bin_cmd!("gate")
    }
}

#[test]
fn test_pass_then_sync_roundtrip() {
    let root = setup_root();
    let file = task_file_name(1);
    write_task(root.path(), TaskLocation::Todo, &file, &base_task(1));

    let db_path = rebuild_cache(root.path());
    let id = extract_task_id(&file);

    run_pass(root.path(), &db_path, &id, Some("Implementation done."));
    run_pass(root.path(), &db_path, &id, Some("Ready for red."));
    run_pass(root.path(), &db_path, &id, Some("Ready for blue."));
    run_pass(root.path(), &db_path, &id, Some("All tests pass."));

    let done_path = root.path().join("done").join(&file);
    assert!(!root.path().join("todo").join(&file).exists());
    assert!(done_path.exists());
    let final_state = sync::resolve_task_state(
        &done_path,
        &fs::read_to_string(&done_path).unwrap(),
        &maenifold_gate::pipeline::load_pipeline_def(None, root.path()).unwrap(),
    )
    .unwrap();
    assert_eq!(final_state.status, "done");
    assert_eq!(final_state.location, TaskLocation::Done);

    resync(root.path(), &db_path);

    let row = read_row(&db_path, &file);
    assert_eq!(row.requirement, "FR-41");
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert_eq!(row.attempts, 0);
}

#[test]
fn test_done_without_frontmatter_stays_pending() {
    let root = setup_root();
    let file = task_file_name(10);
    write_task(root.path(), TaskLocation::Done, &file, &base_task(10));
    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "pending");
    assert_eq!(row.location, "done");
}

#[test]
fn test_done_frontmatter_controls_status() {
    let root = setup_root();
    let file = task_file_name(11);
    write_task(
        root.path(),
        TaskLocation::Done,
        &file,
        &with_frontmatter("done", 0, with_swe(base_task(11))),
    );
    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
}

#[test]
fn test_todo_zero_sections_stays_pending() {
    let root = setup_root();
    let file = task_file_name(13);
    let path = write_task(root.path(), TaskLocation::Todo, &file, &base_task(13));
    let pipeline = maenifold_gate::pipeline::load_pipeline_def(None, root.path()).unwrap();
    let state =
        sync::resolve_task_state(&path, &fs::read_to_string(&path).unwrap(), &pipeline).unwrap();
    assert_eq!(state.status, "pending");
    assert_eq!(state.location, TaskLocation::Todo);
}

#[test]
fn test_sync_rebuild_legacy_done_file() {
    let root = setup_root();
    let legacy = task_file_name(14);
    write_task(root.path(), TaskLocation::Done, &legacy, &base_task(14));
    let modern = task_file_name(15);
    write_task(
        root.path(),
        TaskLocation::Done,
        &modern,
        &with_frontmatter(
            "done",
            0,
            with_blue_pass(with_red_pass(with_swe(base_task(15)))),
        ),
    );
    let pending = task_file_name(16);
    write_task(root.path(), TaskLocation::Todo, &pending, &base_task(16));

    let db_path = rebuild_cache(root.path());

    let legacy_row = read_row(&db_path, &legacy);
    assert_eq!(legacy_row.status, "pending");
    assert_eq!(legacy_row.location, "done");

    let modern_row = read_row(&db_path, &modern);
    assert_eq!(modern_row.status, "done");
    assert_eq!(modern_row.location, "done");

    let pending_row = read_row(&db_path, &pending);
    assert_eq!(pending_row.status, "pending");
    assert_eq!(pending_row.location, "todo");
}

#[test]
fn test_status_json_reports_legacy_done_file_as_pending() {
    let root = setup_root();
    let file = task_file_name(17);
    write_task(root.path(), TaskLocation::Done, &file, &base_task(17));
    let _db_path = rebuild_cache(root.path());
    let id = extract_task_id(&file);

    let output = gate_command()
        .args(["status", "--root"])
        .arg(root.path())
        .arg("--json")
        .arg(&id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["file"], file);
    assert_eq!(json["location"], "done");
    assert_eq!(json["status"], "pending");
    assert_eq!(json["stages"], Value::Array(vec![]));
    assert!(json["error"].is_null());
}

#[test]
fn test_status_json_reports_sections_without_state_derivation() {
    let root = setup_root();
    write_pipeline(
        root.path(),
        include_str!("../../../assets/pipelines/tdd.json"),
    );
    let file = task_file_name(19);
    write_task(
        root.path(),
        TaskLocation::Todo,
        &file,
        &format!(
            "{}\n## Test\n\nWrote failing coverage.\n\n## Implement\n\n**Verdict**: PASS\n\nImplementation complete.\n",
            base_task(19)
        ),
    );

    let output = gate_command()
        .args(["status", "--root"])
        .arg(root.path())
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    let items = json.as_array().unwrap();
    let status = items.iter().find(|item| item["file"] == file).unwrap();

    assert_eq!(status["location"], "todo");
    assert_eq!(status["status"], "pending");
    assert_eq!(status["stages"], Value::Array(vec![]));
    assert_eq!(
        status["sections"],
        serde_json::json!([
            {
                "heading": "Test",
                "stage": "Test",
                "body": "Wrote failing coverage."
            },
            {
                "heading": "Implement",
                "stage": "Implement",
                "verdict": "PASS",
                "body": "**Verdict**: PASS\n\nImplementation complete."
            }
        ])
    );
}

#[test]
fn test_frontmatter_attempts_drive_sync_for_done_file() {
    let root = setup_root();
    let file = task_file_name(18);
    write_task(
        root.path(),
        TaskLocation::Done,
        &file,
        &with_frontmatter(
            "done",
            1,
            with_blue_pass(with_red_pass(with_swe(base_task(18)))).replace(
                "**Verdict**: PASS\n\nAll tests pass.",
                "**Verdict**: FAIL\n\nRegression still present.",
            ),
        ),
    );

    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);

    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert_eq!(row.attempts, 1);
}
