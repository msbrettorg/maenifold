mod support;

use predicates::prelude::*;
use std::thread;
use support::{
    gate_command, gate_db_path, gitignore_path, new_pipeline_root, read_task, status_json,
    sync_gate, write_pending_task,
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

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "pending gate",
        ])
        .assert()
        .success();

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
        gate_command(root.path())
            .args([
                "pass",
                "T-1.1",
                "--root",
                root.path().to_str().expect("root utf8"),
                "--agent",
                "test-agent",
                "--notes",
                notes,
            ])
            .assert()
            .success();
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
        gate_command(root.path())
            .args([
                "pass",
                "T-1.1",
                "--root",
                root.path().to_str().expect("root utf8"),
                "--agent",
                "test-agent",
                "--notes",
                notes,
            ])
            .assert()
            .success();
    }

    assert!(!todo_path.exists());
    assert!(support::done_task_path(root.path(), "T-1.1").exists());

    gate_command(root.path())
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "reopen for rework",
        ])
        .assert()
        .success();

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

#[test]
fn fail_requires_agent_flag() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--notes",
            "needs rework",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--agent"));
}

#[test]
fn fail_returns_sdd_task_from_blue_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "pending gate",
        ])
        .assert()
        .success();

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "red approved",
        ])
        .assert()
        .success();

    gate_command(root.path())
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "blue verification failed",
        ])
        .assert()
        .success();

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("\"attempts\":1") || rendered.contains("\"attempts\": 1"));
    let task_text = read_task(&task);
    assert!(task_text.contains("## Blue-Team"));
    assert!(task_text.contains("FAIL"));
}

#[test]
fn fail_from_pending_dispatch_state_returns_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "pending gate",
        ])
        .assert()
        .success();

    gate_command(root.path())
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "test-agent",
            "--notes",
            "cannot fail before swe runs",
        ])
        .assert()
        .success();

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("\"attempts\":1") || rendered.contains("\"attempts\": 1"));
    let task_text = read_task(&task);
    assert!(task_text.contains("## SWE"));
    assert!(task_text.contains("FAIL"));
}

#[test]
fn sync_rebuild_writes_db_under_dot_gate_and_updates_gitignore_idempotently() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.2");

    gate_command(root.path())
        .args([
            "sync",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--rebuild",
        ])
        .assert()
        .success();

    assert!(gate_db_path(root.path()).exists());
    assert!(!root.path().join("gate.db").exists());

    let first_gitignore =
        std::fs::read_to_string(gitignore_path(root.path())).expect("read .gitignore");
    assert!(first_gitignore.contains(".gate/"));

    gate_command(root.path())
        .args([
            "sync",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--rebuild",
        ])
        .assert()
        .success();

    let second_gitignore = std::fs::read_to_string(gitignore_path(root.path()))
        .expect("read .gitignore after second sync");
    let occurrences = second_gitignore
        .lines()
        .filter(|line| line.trim() == ".gate/")
        .count();
    assert_eq!(occurrences, 1);
}
