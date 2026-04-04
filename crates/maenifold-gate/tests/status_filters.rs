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

#[test]
fn status_includes_nested_done_legacy_tasks() {
    let root = new_pipeline_root();
    fs::create_dir_all(root.path().join("done").join("legacy")).expect("create done legacy");
    fs::write(
        root.path().join("done").join("legacy").join("T-90.18.md"),
        "---\nid: T-90.18\ntitle: Legacy done task\npipeline: sdd\nstate: done\n---\n# Legacy done task\n",
    )
    .expect("write nested legacy done task");

    sync_gate(root.path());

    let rows = status_json(root.path())
        .as_array()
        .expect("status array")
        .clone();
    let task = rows
        .iter()
        .find(|row| row["taskId"] == "T-90.18")
        .expect("nested done task should be visible");
    assert_eq!(task["location"], "done");
    assert!(
        task["path"]
            .as_str()
            .expect("path string")
            .ends_with("done/legacy/T-90.18.md")
    );
}
