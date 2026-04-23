mod support;

use predicates::prelude::*;
use support::{
    gate_command, gate_db_path, gitignore_path, new_pipeline_root, read_task, status_json,
    write_pending_task,
};

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
