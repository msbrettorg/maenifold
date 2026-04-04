mod support;

use predicates::prelude::*;
use support::{gate_command, new_pipeline_root, write_pending_task};

#[test]
fn history_renders_full_stage_journey_for_done_task() {
    let root = new_pipeline_root();
    let task_id = "T-90.14";
    write_pending_task(root.path(), task_id);

    gate_command(root.path())
        .args([
            "append",
            task_id,
            "--root",
            root.path().to_str().expect("root utf8"),
            "--agent",
            "swe",
            "--notes",
            "initial implementation started",
            "--model",
            "claude-sonnet-4.5",
        ])
        .assert()
        .success();

    for (command, notes) in [
        ("pass", "swe completed"),
        ("fail", "red-team requested rework"),
        ("pass", "swe remediation completed"),
        ("pass", "red-team approved"),
        ("pass", "blue-team re-queued"),
        ("pass", "blue-team verified"),
    ] {
        gate_command(root.path())
            .args([
                command,
                task_id,
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

    gate_command(root.path())
        .args(["history", task_id, "--root"])
        .arg(root.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("T-90.14 history")
                .and(predicate::str::contains("pipeline: sdd"))
                .and(predicate::str::contains("location: done"))
                .and(predicate::str::contains(
                    "stage=pending event=append agent=swe",
                ))
                .and(predicate::str::contains("model: claude-sonnet-4.5"))
                .and(predicate::str::contains(
                    "stage=pending event=PASS agent=test-agent",
                ))
                .and(predicate::str::contains(
                    "stage=swe event=FAIL agent=test-agent",
                ))
                .and(predicate::str::contains(
                    "stage=blue-team event=PASS agent=test-agent",
                ))
                .and(predicate::str::contains("notes: blue-team re-queued"))
                .and(predicate::str::contains("notes: red-team requested rework"))
                .and(predicate::str::contains("notes: blue-team verified")),
        );
}

#[test]
fn history_reports_when_task_has_no_gate_records() {
    let root = new_pipeline_root();
    let task_id = "T-90.15";
    write_pending_task(root.path(), task_id);

    gate_command(root.path())
        .args(["history", task_id, "--root"])
        .arg(root.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No gate history recorded yet."));
}
