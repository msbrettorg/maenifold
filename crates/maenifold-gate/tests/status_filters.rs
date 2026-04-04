mod support;

use predicates::prelude::*;
use support::{gate_command, new_pipeline_root, status_json, sync_gate, write_pending_task};

#[test]
fn status_supports_ready_and_state_filters() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-90.13-ready");
    write_pending_task(root.path(), "T-90.13-active");

    sync_gate(root.path());
    gate_command(root.path())
        .args(["pass", "T-90.13-active", "--root"])
        .arg(root.path())
        .args(["--agent", "swe", "--notes", "advance active task"])
        .assert()
        .success();

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .arg("--ready")
        .assert()
        .success()
        .stdout(predicates::str::contains("T-90.13-ready"))
        .stdout(predicates::str::contains("T-90.13-active").not());

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .args(["--state", "swe"])
        .assert()
        .success()
        .stdout(predicates::str::contains("T-90.13-active"))
        .stdout(predicates::str::contains("T-90.13-ready").not());
}

#[test]
fn status_excludes_done_by_default_and_includes_done_with_all() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-92.3-pending");
    write_pending_task(root.path(), "T-92.3-done");

    sync_gate(root.path());
    for (agent, notes) in [
        ("swe", "advance to swe"),
        ("red-team", "advance to red-team"),
        ("blue-team", "advance to blue-team"),
        ("blue-team", "complete task"),
    ] {
        gate_command(root.path())
            .args(["pass", "T-92.3-done", "--root"])
            .arg(root.path())
            .args(["--agent", agent, "--notes", notes])
            .assert()
            .success();
    }

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("T-92.3-pending"))
        .stdout(predicates::str::contains("T-92.3-done").not());

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .arg("--all")
        .assert()
        .success()
        .stdout(predicates::str::contains("T-92.3-pending"))
        .stdout(predicates::str::contains("T-92.3-done"));

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .args(["--state", "done"])
        .assert()
        .success()
        .stdout(predicates::str::contains("T-92.3-done"))
        .stdout(predicates::str::contains("T-92.3-pending").not());

    let all_status = status_json(root.path());
    let rows = all_status.as_array().expect("status array");
    assert!(rows.iter().any(|row| row["taskId"] == "T-92.3-done"));
}
