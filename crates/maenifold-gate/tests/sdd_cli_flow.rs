mod support;

use predicates::prelude::*;
use std::thread;
use support::{
    fail_task, gate_command, new_pipeline_root, pass_task, read_task, status_json, sync_gate,
    write_pending_task,
};

#[test]
fn status_reports_new_todo_task_as_pending() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1");
    sync_gate(root.path());

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("T-1.1"));
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("sdd"));
}

#[test]
fn append_records_swe_work_without_advancing_state() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

    pass_task(root.path(), "T-1.1", "test-agent", "pending gate");

    gate_command(root.path())
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "swe",
            "--notes",
            "implemented auth flow",
        ])
        .assert()
        .success();

    let task_text = read_task(&task);
    assert!(task_text.contains("## SWE"));
    assert!(task_text.contains("**Pipeline**: sdd"));

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("swe"));
}

#[test]
fn concurrent_append_operations_preserve_both_sections() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");
    let left_root = root.path().to_path_buf();
    let right_root = root.path().to_path_buf();

    let left = thread::spawn(move || {
        gate_command(&left_root)
            .args([
                "append",
                "T-1.1",
                "--root",
                left_root.to_str().expect("root utf8"),
                "--agent",
                "swe",
                "--notes",
                "first concurrent append",
            ])
            .assert()
            .success();
    });

    let right = thread::spawn(move || {
        gate_command(&right_root)
            .args([
                "append",
                "T-1.1",
                "--root",
                right_root.to_str().expect("root utf8"),
                "--agent",
                "swe",
                "--notes",
                "second concurrent append",
            ])
            .assert()
            .success();
    });

    left.join().expect("join first append");
    right.join().expect("join second append");

    let task_text = read_task(&task);
    assert!(task_text.contains("first concurrent append"));
    assert!(task_text.contains("second concurrent append"));
}

#[test]
fn append_requires_agent_flag() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--notes",
            "implemented auth flow",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--agent"));
}

#[test]
fn append_on_done_task_fails_with_explicit_message() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

    for notes in [
        "pending gate",
        "swe completed",
        "red approved",
        "blue verified",
    ] {
        pass_task(root.path(), "T-1.1", "test-agent", notes);
    }

    assert!(!task.exists());

    gate_command(root.path())
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "post completion note",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already done").and(predicate::str::contains("append")));
}

#[test]
fn pass_requires_agent_flag() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--notes",
            "swe completed",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--agent"));
}

#[test]
fn fail_on_done_task_reopens_it_in_todo_pending() {
    let root = new_pipeline_root();
    let todo_path = write_pending_task(root.path(), "T-1.1");

    for notes in [
        "pending gate",
        "swe completed",
        "red approved",
        "blue verified",
    ] {
        pass_task(root.path(), "T-1.1", "test-agent", notes);
    }

    assert!(!todo_path.exists());
    assert!(support::done_task_path(root.path(), "T-1.1").exists());

    fail_task(root.path(), "T-1.1", "test-agent", "reopen for rework");

    let reopened_path = support::task_path(root.path(), "T-1.1");
    assert!(reopened_path.exists());
    assert!(!support::done_task_path(root.path(), "T-1.1").exists());

    let task_text = read_task(&reopened_path);
    assert!(task_text.contains("## Done"));
    assert!(task_text.contains("FAIL"));

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("\"attempts\":1") || rendered.contains("\"attempts\": 1"));
}
