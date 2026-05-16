use super::*;
use std::fs;
use std::path::Path;

fn drive_to_done(root: &Path, task_id: &str) {
    for _ in 0..16 {
        let row = task_status(root, task_id);
        if row["location"] == "done" {
            return;
        }
        GateLifecycle::pass(note(root, task_id, "pm", "advance")).expect("pass");
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
    fs::rename(
        task_path(root, task_id, "done"),
        task_path(root, task_id, "todo"),
    )
    .expect("move done task back to todo");
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
fn reopen_errors_when_task_is_in_done_location() {
    let root = new_root();
    write_task(root.path(), "T-325.15", "sdd");
    drive_to_done(root.path(), "T-325.15");

    let error = GateLifecycle::reopen(note(root.path(), "T-325.15", "pm", "reopening task"))
        .expect_err("reopen rejects done location");

    assert!(error.to_string().contains("not in todo"));
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
