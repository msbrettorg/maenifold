use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use maenifold_gate::{
    StatusArgs, SyncArgs, TaskLocation, TaskState,
    db::{GateDb, extract_task_id},
    parser,
    pass::{self, PassArgs},
    status, sync,
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

fn rebuild_cache(root: &Path) -> PathBuf {
    let db_path = root.join("pipeline.db");
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

fn gate_command() -> Command {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_gate") {
        Command::new(path)
    } else {
        cargo_bin_cmd!("gate")
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
        &extract_task_id(&blue_file),
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
        pipeline_path: None,
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
fn test_status_single_id() {
    let root = setup_root();
    let file = task_file_name(1);
    write_task(
        root.path(),
        TaskLocation::Todo,
        &file,
        &with_red_pass(with_swe(base_task(1))),
    );

    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "blue");

    let args = StatusArgs {
        db: Some(db_path),
        root: root.path().to_path_buf(),
        json: false,
        id: Some(extract_task_id(&file)),
        pipeline_path: None,
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
        root: root.path().to_path_buf(),
        json: false,
        id: None,
        pipeline_path: None,
    };

    status::run_status(&args, root.path()).unwrap();
}

#[test]
#[allow(deprecated)]
fn test_pass_then_sync_roundtrip() {
    let root = setup_root();
    let file = task_file_name(1);
    write_task(root.path(), TaskLocation::Todo, &file, &base_task(1));

    let db_path = rebuild_cache(root.path());
    let id = extract_task_id(&file);
    for note in [
        "Implementation done.",
        "Ready for red.",
        "Ready for blue.",
        "All tests pass.",
    ] {
        run_pass(root.path(), &db_path, &id, Some(note));
    }

    let done_path = root.path().join("done").join(&file);
    assert!(!root.path().join("todo").join(&file).exists());
    assert!(done_path.exists());
    let final_state = parser::parse_task_file(&done_path).unwrap();
    assert_eq!(final_state.effective_status(), "done");
    assert_eq!(final_state.location, TaskLocation::Done);

    let sync_args = SyncArgs {
        db: Some(db_path.clone()),
        root: root.path().to_path_buf(),
        rebuild: false,
        pipeline_path: None,
    };
    sync::run_sync(&sync_args, root.path()).unwrap();

    let row = read_row(&db_path, &file);
    assert_eq!(row.requirement, "FR-41");
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert_eq!(row.attempts, 0);
}

// T-54.2: Location-authoritative fallback tests

#[test]
#[allow(deprecated)]
fn test_done_zero_sections_effective_done() {
    let root = setup_root();
    let file = task_file_name(10);
    let path = write_task(root.path(), TaskLocation::Done, &file, &base_task(10));
    assert!(parser::parse_task_file(&path).is_err());
    let state = TaskState::done_override(path);
    assert_eq!(state.effective_status(), "done");
    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
}

#[test]
#[allow(deprecated)]
fn test_done_swe_only_effective_done() {
    let root = setup_root();
    let file = task_file_name(11);
    let path = write_task(
        root.path(),
        TaskLocation::Done,
        &file,
        &with_swe(base_task(11)),
    );
    let parsed = parser::parse_task_file(&path).unwrap();
    assert_eq!(parsed.effective_status(), "done");
    let state = TaskState::done_override(path);
    assert_eq!(state.effective_status(), "done");
    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
}

#[test]
#[allow(deprecated)]
fn test_done_swe_red_pass_no_blue_effective_done() {
    let root = setup_root();
    let file = task_file_name(12);
    let path = write_task(
        root.path(),
        TaskLocation::Done,
        &file,
        &with_red_pass(with_swe(base_task(12))),
    );
    let state = parser::parse_task_file(&path).unwrap();
    assert_eq!(state.effective_status(), "done");
    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);
    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
}

#[test]
#[allow(deprecated)]
fn test_todo_zero_sections_stays_pending() {
    let root = setup_root();
    let file = task_file_name(13);
    let path = write_task(root.path(), TaskLocation::Todo, &file, &base_task(13));
    let state = parser::parse_task_file(&path).unwrap();
    assert_eq!(state.status, "pending");
    assert_eq!(state.effective_status(), "pending");
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
        &with_blue_pass(with_red_pass(with_swe(base_task(15)))),
    );
    let pending = task_file_name(16);
    write_task(root.path(), TaskLocation::Todo, &pending, &base_task(16));

    let db_path = rebuild_cache(root.path());

    let legacy_row = read_row(&db_path, &legacy);
    assert_eq!(legacy_row.status, "done");
    assert_eq!(legacy_row.location, "done");

    let modern_row = read_row(&db_path, &modern);
    assert_eq!(modern_row.status, "done");
    assert_eq!(modern_row.location, "done");

    let pending_row = read_row(&db_path, &pending);
    assert_eq!(pending_row.status, "pending");
    assert_eq!(pending_row.location, "todo");
}

#[test]
fn test_status_json_reports_done_override_for_legacy_done_file() {
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
    assert_eq!(json["status"], "done");
    assert_eq!(json["stages"], Value::Array(vec![]));
    assert_eq!(json["error"], "incomplete audit trail (location override)");
}

#[test]
fn test_status_json_reports_dynamic_pipeline_stages() {
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
    assert_eq!(status["status"], "verify");
    assert_eq!(
        status["stages"],
        serde_json::json!([
            { "name": "test", "verdict": null },
            { "name": "implement", "verdict": "PASS" }
        ])
    );
    let object = status.as_object().unwrap();
    assert!(!object.contains_key("swe"));
    assert!(!object.contains_key("red"));
    assert!(!object.contains_key("blue"));
}

#[test]
fn test_done_override_preserves_attempts_during_sync() {
    let root = setup_root();
    let file = task_file_name(18);
    write_task(
        root.path(),
        TaskLocation::Done,
        &file,
        &with_blue_pass(with_red_pass(with_swe(base_task(18)))).replace(
            "**Verdict**: PASS\n\nAll tests pass.",
            "**Verdict**: FAIL\n\nRegression still present.",
        ),
    );

    let db_path = rebuild_cache(root.path());
    let row = read_row(&db_path, &file);

    assert_eq!(row.status, "done");
    assert_eq!(row.location, "done");
    assert_eq!(row.attempts, 1);
}
