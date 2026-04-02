mod support;

use predicates::prelude::*;
use support::{
    gate_command, new_pipeline_root, read_task, status_json,
    write_legacy_task_without_pipeline_binding, write_pending_task, write_task_with_mismatched_digest,
    write_task_without_pipeline_digest,
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
fn fail_returns_sdd_task_from_blue_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");

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
        .success();

    gate_command(root.path())
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
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
fn illegal_fail_from_pending_dispatch_state_returns_error_and_leaves_files_unchanged() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1");
    let before = read_task(&task);

    gate_command(root.path())
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--notes",
            "cannot fail before swe runs",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("illegal").or(predicate::str::contains("fail")));

    assert_eq!(read_task(&task), before);
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
