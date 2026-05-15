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
    write_task(root.path(), "T-237.1", "sdd");
    let path = task_path(root.path(), "T-237.1", "todo");

    GateLifecycle::append(note(root.path(), "T-237.1", "swe", "captured progress"))
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
    write_task(root.path(), "T-237.2", "sdd");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.2", "pm", "ready")).expect("pass succeeds");
    assert_eq!(after_pass["state"], "swe");

    let after_fail = GateLifecycle::fail(note(root.path(), "T-237.2", "swe", "needs spec work"))
        .expect("fail succeeds");
    assert_eq!(after_fail["state"], "pending");

    let text = fs::read_to_string(task_path(root.path(), "T-237.2", "todo")).expect("read task");
    assert!(text.contains("- Outcome: PASS"));
    assert!(text.contains("- Outcome: FAIL"));
    assert!(text.contains("- Stage ID: swe"));
}

#[test]
fn pass_with_matching_expected_stage_advances_to_next_stage() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let after_pass = GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "pending",
        "ready",
    ))
    .expect("pass succeeds when expected stage matches");

    assert_eq!(after_pass["state"], "swe");
}

#[test]
fn pass_with_wrong_expected_stage_returns_error_and_preserves_state() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let error = GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "swe",
        "stale snapshot",
    ))
    .expect_err("pass should reject stale expected stage");

    assert!(error.to_string().contains("expected stage"));
    assert_eq!(single_status(root.path())["state"], "pending");
}

#[test]
fn fail_with_wrong_expected_stage_returns_error_and_preserves_state() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let error = GateLifecycle::fail(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "swe",
        "stale snapshot",
    ))
    .expect_err("fail should reject stale expected stage");

    assert!(error.to_string().contains("expected stage"));
    assert_eq!(single_status(root.path())["state"], "pending");
}

#[test]
fn terminal_pass_moves_task_to_done() {
    let root = new_root();
    write_task(root.path(), "T-237.3", "test");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.3", "pm", "complete")).expect("pass");

    assert_eq!(after_pass["state"], "done");
    assert!(!task_path(root.path(), "T-237.3", "todo").exists());
    assert!(task_path(root.path(), "T-237.3", "done").exists());
}

#[test]
fn read_commands_do_not_mutate_task_files() {
    let root = new_root();
    write_task(root.path(), "T-237.4", "sdd");
    let path = task_path(root.path(), "T-237.4", "todo");
    let before = fs::read_to_string(&path).expect("read before");

    GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
        summary: false,
    })
    .expect("status");
    GateLifecycle::history(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.4".to_string(),
    })
    .expect("history");
    GateLifecycle::task_body(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.4".to_string(),
    })
    .expect("task body");

    let after = fs::read_to_string(&path).expect("read after");
    assert_eq!(after, before);
}

#[path = "lifecycle_sync_status_tests.rs"]
mod sync_status;

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
        expected_stage: None,
    }
}

fn note_with_expected_stage(
    root: &Path,
    task_id: &str,
    agent: &str,
    expected_stage: &str,
    notes: &str,
) -> StageNoteArgs {
    StageNoteArgs {
        context: context(root),
        task_id: task_id.to_string(),
        agent: agent.to_string(),
        notes: notes.to_string(),
        model: Some("test-model".to_string()),
        expected_stage: Some(expected_stage.to_string()),
    }
}

fn single_status(root: &Path) -> serde_json::Value {
    let rows = GateLifecycle::status(StatusArgs {
        context: context(root),
        filter: default_filter(),
        summary: false,
    })
    .expect("status");
    rows.as_array().expect("array")[0].clone()
}

#[test]
fn status_summary_returns_compact_per_task_rows() {
    let root = new_root();
    for index in 1..=10 {
        write_task(root.path(), &format!("T-303.{index}"), "sdd");
    }
    GateLifecycle::pass(note(root.path(), "T-303.1", "pm", "ready")).expect("pass");
    GateLifecycle::pass(note(root.path(), "T-303.2", "pm", "ready")).expect("pass");
    GateLifecycle::fail(note(root.path(), "T-303.2", "swe", "retry")).expect("fail");

    let rows = GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
        summary: true,
    })
    .expect("status summary");
    let rows = rows.as_array().expect("array");

    assert_eq!(rows.len(), 10);
    assert!(serde_json::to_vec(rows).expect("serialize").len() < 1024);
    for row in rows {
        let object = row.as_object().expect("object");
        assert_eq!(object.len(), 6);
        assert!(object.contains_key("taskId"));
        assert!(object.contains_key("state"));
        assert!(object.contains_key("location"));
        assert!(object.contains_key("attempts"));
        assert!(object.contains_key("linked"));
        assert!(object.contains_key("sprint"));
    }
}

#[test]
fn status_without_summary_keeps_full_task_rows() {
    let root = new_root();
    write_task(root.path(), "T-303.99", "sdd");

    let rows = GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
        summary: false,
    })
    .expect("status");
    let row = rows.as_array().expect("array")[0]
        .as_object()
        .expect("object");

    assert_eq!(row.len(), 18);
    assert!(row.contains_key("title"));
    assert!(row.contains_key("path"));
    assert!(row.contains_key("stages"));
    assert!(row.contains_key("pipeline"));
    assert!(row.contains_key("ready"));
    assert!(row.contains_key("blocked"));
    assert!(row.contains_key("blocked_reason"));
    assert!(row.contains_key("superseded_by"));
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
