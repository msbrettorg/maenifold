use super::*;
use std::fs;
use std::path::Path;

fn drive_to_done(root: &Path, task_id: &str) {
    for _ in 0..16 {
        let row = task_status(root, task_id);
        if row["location"] == "done" {
            return;
        }
        let stage = row["state"].as_str().expect("state");
        GateLifecycle::pass(note_with_expected_stage(
            root, task_id, "pm", stage, "advance",
        ))
        .expect("pass");
    }
    panic!("{task_id} did not reach done");
}

fn task_status(root: &Path, task_id: &str) -> serde_json::Value {
    let rows = GateLifecycle::status(StatusArgs {
        context: context(root),
        filter: default_filter(),
        summary: false,
    })
    .expect("status");
    rows.as_array()
        .expect("array")
        .iter()
        .find(|r| r["taskId"] == task_id)
        .cloned()
        .expect("find task")
}

fn reopen_done_task(root: &Path, task_id: &str) -> serde_json::Value {
    drive_to_done(root, task_id);
    GateLifecycle::reopen(note(root, task_id, "pm", "reopening task")).expect("reopen")
}

fn assert_reopened(row: &serde_json::Value) {
    assert_eq!(row["state"], "pending");
    assert_eq!(row["location"], "todo");
    assert!(row["attempts"].as_i64().expect("attempts") >= 1);
}

#[test]
fn reopen_after_terminal_tdd_resets_to_pending() {
    let root = new_root();
    write_task(root.path(), "T-325.10", "tdd");

    let reopened = reopen_done_task(root.path(), "T-325.10");

    assert_reopened(&reopened);
}

#[test]
fn reopen_after_terminal_sdd_resets_to_pending() {
    let root = new_root();
    write_task(root.path(), "T-325.11", "sdd");

    let reopened = reopen_done_task(root.path(), "T-325.11");

    assert_reopened(&reopened);
}

#[test]
fn reopen_after_terminal_gsd_resets_to_pending() {
    let root = new_root();
    write_task(root.path(), "T-325.12", "gsd");

    let reopened = reopen_done_task(root.path(), "T-325.12");

    assert_reopened(&reopened);
}

#[test]
fn reopen_after_terminal_slc_resets_to_pending() {
    let root = new_root();
    write_task(root.path(), "T-325.13", "slc");

    let reopened = reopen_done_task(root.path(), "T-325.13");

    assert_reopened(&reopened);
}

#[test]
fn reopen_after_terminal_lightweight_resets_to_pending() {
    let root = new_root();
    write_task(root.path(), "T-325.14", "lightweight");

    let reopened = reopen_done_task(root.path(), "T-325.14");

    assert_reopened(&reopened);
}

#[test]
fn reopen_done_located_tdd_task_moves_to_todo() {
    let root = new_root();
    write_task(root.path(), "T-325.20", "tdd");
    drive_to_done(root.path(), "T-325.20");

    let done_row = task_status(root.path(), "T-325.20");
    assert_eq!(done_row["location"], "done");

    let before_history = GateLifecycle::history(TaskArgs {
        context: context(root.path()),
        task_id: "T-325.20".to_string(),
    })
    .expect("history before reopen");
    let before_history_len = before_history.as_array().expect("history array").len();

    let reopened =
        GateLifecycle::reopen(note(root.path(), "T-325.20", "pm", "reopening done task"))
            .expect("reopen should move done task back to todo");

    assert_reopened(&reopened);
    assert!(task_path(root.path(), "T-325.20", "todo").exists());
    assert!(!task_path(root.path(), "T-325.20", "done").exists());

    let after_history = GateLifecycle::history(TaskArgs {
        context: context(root.path()),
        task_id: "T-325.20".to_string(),
    })
    .expect("history after reopen");
    let after_history = after_history.as_array().expect("history array");
    assert_eq!(after_history.len(), before_history_len + 1);
    let reopen_record = after_history.last().expect("reopen record");
    assert_eq!(reopen_record["stage"], "reopened");
    assert_eq!(reopen_record["event"], "reopen");
}

#[test]
fn reopen_done_located_sdd_task_preserves_slug_filename() {
    let root = new_root();
    let slug_path = root.path().join("todo").join("T-325.21-my-cool-slug.md");
    fs::write(
        &slug_path,
        "---\nid: T-325.21\ntitle: My cool slug\npipeline: sdd\n---\n# My cool slug\n\n## Acceptance Criteria\n\n1. test task\n",
    )
    .expect("write slug task");
    drive_to_done(root.path(), "T-325.21");

    let reopened = GateLifecycle::reopen(note(
        root.path(),
        "T-325.21",
        "pm",
        "reopening done slug task",
    ))
    .expect("reopen should restore slug filename");

    assert_reopened(&reopened);
    assert!(slug_path.exists());
    assert!(!task_path(root.path(), "T-325.21", "todo").exists());
    let reopened_path = std::path::PathBuf::from(reopened["path"].as_str().expect("path"));
    assert_eq!(
        fs::canonicalize(&reopened_path).expect("canonicalize reopened"),
        fs::canonicalize(&slug_path).expect("canonicalize slug"),
    );
}

#[test]
fn reopen_todo_located_terminal_task_still_works() {
    let root = new_root();
    write_task(root.path(), "T-325.22", "tdd");
    drive_to_done(root.path(), "T-325.22");

    fs::rename(
        task_path(root.path(), "T-325.22", "done"),
        task_path(root.path(), "T-325.22", "todo"),
    )
    .expect("move done task back to todo");

    let reopened = GateLifecycle::reopen(note(root.path(), "T-325.22", "pm", "reopening task"))
        .expect("reopen");

    assert_reopened(&reopened);
}

#[test]
fn reopen_errors_when_task_is_mid_pipeline() {
    let root = new_root();
    write_task(root.path(), "T-325.16", "sdd");
    GateLifecycle::pass(note(root.path(), "T-325.16", "pm", "advance")).expect("pass");

    let error = GateLifecycle::reopen(note(root.path(), "T-325.16", "pm", "reopening task"))
        .expect_err("reopen rejects non-terminal state");

    assert!(error.to_string().contains("not in a terminal state"));
}

#[test]
fn pass_after_reopen_advances_normally() {
    let root = new_root();
    write_task(root.path(), "T-325.17", "sdd");
    let reopened = reopen_done_task(root.path(), "T-325.17");
    assert_eq!(reopened["state"], "pending");

    let advanced = GateLifecycle::pass(note(root.path(), "T-325.17", "pm", "advance"))
        .expect("pass after reopen");

    assert_eq!(advanced["state"], "swe");
}
