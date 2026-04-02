mod support;

use predicates::prelude::*;
use support::{
    done_task_path, gate_command, new_pipeline_root, read_task, status_json, task_path,
    write_bound_task, write_legacy_task_without_pipeline_binding, write_pending_task,
};

#[test]
fn status_reports_new_todo_task_as_pending() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("T-1.1"));
    assert!(rendered.contains("pending"));
}

#[test]
fn append_records_swe_work_without_advancing_state() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--agent",
            "swe",
            "--notes",
            "implemented auth flow",
        ])
        .assert()
        .success();

    let task_text = read_task(&task);
    assert!(task_text.contains("## SWE Remediation"));

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending") || rendered.contains("swe"));
}

#[test]
fn pass_advances_sdd_task_from_swe_to_red() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("red"));
}

#[test]
fn reject_returns_sdd_task_from_red_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "reject",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red found drift",
        ])
        .assert()
        .success();

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("\"attempts\":1") || rendered.contains("\"attempts\": 1"));
    assert!(read_task(&task).contains("## Red-team Audit"));
}

#[test]
fn pass_advances_sdd_task_from_red_to_blue() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red approved",
        ])
        .assert()
        .success();

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("blue"));
}

#[test]
fn fail_returns_sdd_task_from_blue_to_pending_and_increments_attempts() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red approved",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "blue verification failed",
        ])
        .assert()
        .success();

    let output = status_json(root.path(), "sdd");
    let rendered = serde_json::to_string(&output).expect("render status JSON");
    assert!(rendered.contains("pending"));
    assert!(rendered.contains("\"attempts\":1") || rendered.contains("\"attempts\": 1"));
}

#[test]
fn final_pass_moves_sdd_task_from_todo_to_done() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red approved",
        ])
        .assert()
        .success();

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "blue approved",
        ])
        .assert()
        .success();

    assert!(!task_path(root.path(), "T-1.1").exists());
    assert!(done_task_path(root.path(), "T-1.1").exists());
}

#[test]
fn illegal_reject_from_swe_returns_error_and_leaves_files_unchanged() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1", "sdd");
    let before = read_task(&task);

    gate_command()
        .args([
            "reject",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "illegal red verdict before red stage",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("illegal").or(predicate::str::contains("reject")));

    assert_eq!(read_task(&task), before);
}

#[test]
fn illegal_pass_from_done_returns_error_and_leaves_files_unchanged() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    for note in ["swe completed", "red approved", "blue approved"] {
        gate_command()
            .args([
                "pass",
                "T-1.1",
                "--root",
                root.path().to_str().expect("root utf8"),
                "--pipeline",
                "sdd",
                "--notes",
                note,
            ])
            .assert()
            .success();
    }

    let done_path = done_task_path(root.path(), "T-1.1");
    let before = read_task(&done_path);

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "illegal terminal pass",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("done").or(predicate::str::contains("terminal")));

    assert_eq!(read_task(&done_path), before);
}

#[test]
fn task_file_requires_explicit_pipeline_binding() {
    let root = new_pipeline_root();
    write_legacy_task_without_pipeline_binding(root.path(), "T-1.1");

    gate_command()
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Pipeline").or(predicate::str::contains("binding")));
}

#[test]
fn task_file_requires_pipeline_digest() {
    let root = new_pipeline_root();
    write_bound_task(root.path(), "T-1.1", "sdd", "");

    gate_command()
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Pipeline Digest").or(predicate::str::contains("digest")));
}

#[test]
fn status_errors_when_pipeline_digest_mismatches_authoritative_asset() {
    let root = new_pipeline_root();
    write_bound_task(
        root.path(),
        "T-1.1",
        "sdd",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );

    gate_command()
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("digest").or(predicate::str::contains("mismatch")));
}

#[test]
fn task_bound_pipeline_cannot_be_overridden_by_cli_filter() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "status",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "lightweight",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pipeline").or(predicate::str::contains("override")));
}

#[test]
fn legacy_task_without_pipeline_binding_is_rejected() {
    let root = new_pipeline_root();
    write_legacy_task_without_pipeline_binding(root.path(), "T-1.1");

    gate_command()
        .args([
            "append",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
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
fn illegal_fail_from_red_returns_error_and_leaves_files_unchanged() {
    let root = new_pipeline_root();
    let task = write_pending_task(root.path(), "T-1.1", "sdd");

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();

    let before = read_task(&task);

    gate_command()
        .args([
            "fail",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red cannot fail in sdd",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fail").or(predicate::str::contains("illegal")));

    assert_eq!(read_task(&task), before);
}

#[test]
fn status_json_reflects_sdd_stage_progression() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-1.1", "sdd");

    let pending = serde_json::to_string(&status_json(root.path(), "sdd")).expect("pending json");
    assert!(pending.contains("pending"));

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "swe completed",
        ])
        .assert()
        .success();
    let red = serde_json::to_string(&status_json(root.path(), "sdd")).expect("red json");
    assert!(red.contains("red"));

    gate_command()
        .args([
            "pass",
            "T-1.1",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--pipeline",
            "sdd",
            "--notes",
            "red approved",
        ])
        .assert()
        .success();
    let blue = serde_json::to_string(&status_json(root.path(), "sdd")).expect("blue json");
    assert!(blue.contains("blue"));
}
