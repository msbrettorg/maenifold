mod support;

use predicates::prelude::*;
use support::{
    gate_command, gate_db_path, gitignore_path, new_pipeline_root, read_task, status_json,
    write_legacy_task_without_pipeline_binding, write_pending_task,
    write_task_with_mismatched_digest, write_task_without_pipeline_digest,
};

#[test]
fn status_reports_new_todo_task_as_pending() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1");

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
    assert!(task_text.contains("## SWE Remediation"));
    assert!(task_text.contains("**Pipeline**: sdd"));
    assert!(task_text.contains("**Pipeline Digest**:"));

    let output = status_json(root.path());
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
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

    for notes in ["swe completed", "red approved", "blue verified"] {
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

    for notes in ["swe completed", "red approved", "blue verified"] {
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
    assert!(task_text.contains("## Blue-team Verification"));
    assert!(task_text.contains("FAIL"));
}

#[test]
fn fail_from_pending_dispatch_state_returns_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

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
    assert!(task_text.contains("## SWE Remediation"));
    assert!(task_text.contains("FAIL"));
}

#[test]
fn task_file_requires_explicit_pipeline_binding() {
    let root = new_pipeline_root();
    write_legacy_task_without_pipeline_binding(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Pipeline").or(predicate::str::contains("binding")));
}

#[test]
fn task_file_requires_pipeline_digest() {
    let root = new_pipeline_root();
    write_task_without_pipeline_digest(root.path(), "T-1.1", "sdd");

    gate_command(root.path())
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Pipeline Digest").or(predicate::str::contains("digest")));
}

#[test]
fn status_errors_when_pipeline_digest_mismatches_authoritative_asset() {
    let root = new_pipeline_root();
    write_task_with_mismatched_digest(root.path(), "T-1.1", "sdd");

    gate_command(root.path())
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("digest").or(predicate::str::contains("mismatch")));
}

#[test]
fn legacy_task_without_pipeline_binding_is_rejected() {
    let root = new_pipeline_root();
    write_legacy_task_without_pipeline_binding(root.path(), "T-1.1");

    gate_command(root.path())
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "swe",
            "--notes",
            "should be rejected",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Pipeline").or(predicate::str::contains("binding")));
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

    let second_gitignore =
        std::fs::read_to_string(gitignore_path(root.path())).expect("read .gitignore after second sync");
    let occurrences = second_gitignore
        .lines()
        .filter(|line| line.trim() == ".gate/")
        .count();
    assert_eq!(occurrences, 1);
}
