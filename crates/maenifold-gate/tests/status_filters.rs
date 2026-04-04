mod support;

use std::fs;

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
