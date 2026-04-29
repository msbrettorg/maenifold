use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{
    GateContext, GateLifecycle, StageNoteArgs, StatusArgs, SyncArgs, TaskArgs, resolve_definition,
};
use crate::status_filter::StatusFilter;

#[test]
fn append_records_canonical_active_stage_without_transition() {
    let root = new_root();
    write_task(root.path(), "T-237.a", "sdd");
    let path = task_path(root.path(), "T-237.a", "todo");

    GateLifecycle::append(note(root.path(), "T-237.a", "swe", "captured progress"))
        .expect("append succeeds");

    let text = fs::read_to_string(path).expect("read task");
    assert!(text.contains("## Pending"));
    assert!(text.contains("- Stage ID: pending"));
    assert!(text.contains("- Record Kind: append"));
    assert!(text.contains("- Agent: swe"));
    assert!(text.contains("- Notes: captured progress"));
    assert!(!text.contains("- Outcome:"));
    let row = single_status(root.path());
    assert_eq!(row["state"], "pending");
}

#[test]
fn pass_and_fail_apply_state_machine_transitions() {
    let root = new_root();
    write_task(root.path(), "T-237.b", "sdd");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.b", "pm", "ready")).expect("pass succeeds");
    assert_eq!(after_pass["state"], "swe");

    let after_fail = GateLifecycle::fail(note(root.path(), "T-237.b", "swe", "needs spec work"))
        .expect("fail succeeds");
    assert_eq!(after_fail["state"], "pending");

    let text = fs::read_to_string(task_path(root.path(), "T-237.b", "todo")).expect("read task");
    assert!(text.contains("- Outcome: PASS"));
    assert!(text.contains("- Outcome: FAIL"));
    assert!(text.contains("- Stage ID: swe"));
}

#[test]
fn terminal_pass_moves_task_to_done() {
    let root = new_root();
    write_task(root.path(), "T-237.c", "test");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.c", "pm", "complete")).expect("pass");

    assert_eq!(after_pass["state"], "done");
    assert!(!task_path(root.path(), "T-237.c", "todo").exists());
    assert!(task_path(root.path(), "T-237.c", "done").exists());
}

#[test]
fn read_commands_do_not_mutate_task_files() {
    let root = new_root();
    write_task(root.path(), "T-237.d", "sdd");
    let path = task_path(root.path(), "T-237.d", "todo");
    let before = fs::read_to_string(&path).expect("read before");

    GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
    })
    .expect("status");
    GateLifecycle::history(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.d".to_string(),
    })
    .expect("history");
    GateLifecycle::task_body(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.d".to_string(),
    })
    .expect("task body");

    let after = fs::read_to_string(&path).expect("read after");
    assert_eq!(after, before);
}

#[test]
fn invalid_transition_and_unknown_task_errors_are_actionable() {
    let root = new_root();
    write_task(root.path(), "T-237.e", "sdd");

    let invalid = GateLifecycle::fail(note(root.path(), "T-237.e", "pm", "bad fail"))
        .expect_err("pending fail is invalid");
    assert!(invalid.to_string().contains("illegal transition"));

    let unknown = GateLifecycle::append(note(root.path(), "T-237.missing", "swe", "notes"))
        .expect_err("unknown task");
    assert!(unknown.to_string().contains("not found"));
    assert!(unknown.to_string().contains("searched directories"));
}

#[test]
fn sync_reports_counts_without_requiring_standalone_gate() {
    let root = new_root();
    write_task(root.path(), "T-237.f", "sdd");

    let value = GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");

    assert_eq!(value["taskCount"], 1);
    assert_eq!(value["synced"], true);
    let metadata = fs::metadata(root.path().join(".gate/gate.db")).expect("gate.db metadata");
    assert!(metadata.len() > 0);
}

#[test]
fn builtin_definitions_are_cycle_free_gate_core_assets() {
    let definition = resolve_definition(Path::new("/"), "slc").expect("slc definition");
    assert_eq!(definition.initial_state().expect("initial"), "pending");
    assert!(definition.find_state("done").expect("done").terminal);
}

#[test]
fn status_error_rows_preserve_task_title_and_pipeline() {
    let root = new_root();
    write_task(root.path(), "T-237.g", "sdd");
    fs::rename(
        task_path(root.path(), "T-237.g", "todo"),
        task_path(root.path(), "T-237.g", "done"),
    )
    .expect("move malformed archive fixture");

    let rows = GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
    })
    .expect("status")
    .as_array()
    .expect("array")
    .clone();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["state"], "error");
    assert_eq!(rows[0]["title"], "T-237.g task");
    assert_eq!(rows[0]["pipeline"], "sdd");
    assert_ne!(rows[0]["title"], "[error] T-237.g");
    assert_ne!(rows[0]["pipeline"], "unknown");
}

fn new_root() -> TempDir {
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("todo")).expect("todo");
    fs::create_dir_all(root.path().join("done")).expect("done");
    root
}

fn write_task(root: &Path, id: &str, pipeline: &str) {
    fs::write(
        task_path(root, id, "todo"),
        format!(
            "---\nid: {id}\ntitle: {id} task\npipeline: {pipeline}\n---\n# {id} task\n\n## Acceptance Criteria\n\n1. test task\n"
        ),
    )
    .expect("write task");
}

fn task_path(root: &Path, id: &str, location: &str) -> PathBuf {
    root.join(location).join(format!("{id}.md"))
}

fn note(root: &Path, task_id: &str, agent: &str, notes: &str) -> StageNoteArgs {
    StageNoteArgs {
        context: context(root),
        task_id: task_id.to_string(),
        agent: agent.to_string(),
        notes: notes.to_string(),
        model: Some("test-model".to_string()),
    }
}

fn single_status(root: &Path) -> serde_json::Value {
    let rows = GateLifecycle::status(StatusArgs {
        context: context(root),
        filter: default_filter(),
    })
    .expect("status");
    rows.as_array().expect("array")[0].clone()
}

fn context(root: &Path) -> GateContext {
    GateContext {
        root: root.to_path_buf(),
        requested_pipeline_name: None,
    }
}

fn default_filter() -> StatusFilter {
    StatusFilter {
        state: None,
        ready_only: false,
        blocked_only: false,
        include_done: true,
    }
}
