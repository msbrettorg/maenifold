mod support;

use support::{
    done_task_path, gate_command, runtime_assets_dir, status_json, task_path, write_bound_task,
};

#[test]
fn sdd_e2e_bootstraps_assets_retries_from_red_and_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.1";
    let todo_path = write_bound_task(root.path(), task_id, "sdd");

    assert!(!runtime_assets_dir(root.path()).exists());

    // pending → swe → red-team, then fail red-team → pending
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "swe completed");
    fail(root.path(), task_id, "red requested rework");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["state"], "pending");
    assert_eq!(task["attempts"], 1);
    assert!(runtime_assets_dir(root.path()).join("sdd.json").exists());
    assert!(runtime_assets_dir(root.path()).join("tdd.json").exists());
    assert!(runtime_assets_dir(root.path()).join("gsd.json").exists());
    assert!(
        runtime_assets_dir(root.path())
            .join("lightweight.json")
            .exists()
    );

    // pending → swe → red-team → blue-team → done
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "swe remediation completed");
    pass(root.path(), task_id, "red approved");
    pass(root.path(), task_id, "blue verified");

    assert!(!todo_path.exists());
    assert!(done_task_path(root.path(), task_id).exists());
    let final_status = status_json(root.path());
    let task = find_task(&final_status, task_id);
    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["state"], "done");
}

#[test]
fn tdd_e2e_retries_from_red_team_and_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.2";
    let todo_path = write_bound_task(root.path(), task_id, "tdd");

    // TDD order: pending → blue-team → swe → red-team → done
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "blue-team: tests written");
    pass(root.path(), task_id, "swe: implementation completed");
    fail(root.path(), task_id, "red-team: audit failed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["state"], "pending");
    assert_eq!(task["attempts"], 1);
    assert!(task_path(root.path(), task_id).exists());

    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "blue-team: tests re-verified");
    pass(root.path(), task_id, "swe: remediation completed");
    pass(root.path(), task_id, "red-team: approved");

    assert!(!todo_path.exists());
    assert!(done_task_path(root.path(), task_id).exists());
    let final_status = status_json(root.path());
    let task = find_task(&final_status, task_id);
    assert_eq!(task["pipeline"], "tdd");
    assert_eq!(task["state"], "done");
}

#[test]
fn gsd_e2e_retries_from_verify_and_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.3";
    write_bound_task(root.path(), task_id, "gsd");

    // pending → research → plan → execute → verify, then fail verify → execute
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "research done");
    pass(root.path(), task_id, "plan done");
    pass(root.path(), task_id, "execute done");
    fail(root.path(), task_id, "verify failed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "gsd");
    assert_eq!(task["state"], "execute");
    assert_eq!(task["attempts"], 0);
    assert!(task_path(root.path(), task_id).exists());

    pass(root.path(), task_id, "execute remediation done");
    pass(root.path(), task_id, "verify passed");

    let final_status = status_json(root.path());
    let task = find_task(&final_status, task_id);
    assert_eq!(task["state"], "done");
    assert!(done_task_path(root.path(), task_id).exists());
}

#[test]
fn lightweight_e2e_retries_from_blue_and_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.4";
    write_bound_task(root.path(), task_id, "lightweight");

    // pending → swe → blue-team, then fail blue-team → pending
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "swe completed");
    fail(root.path(), task_id, "blue failed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "lightweight");
    assert_eq!(task["state"], "pending");
    assert_eq!(task["attempts"], 1);

    // pending → swe → blue-team → done
    pass(root.path(), task_id, "pending gate");
    pass(root.path(), task_id, "swe remediation completed");
    pass(root.path(), task_id, "blue verified");

    let final_status = status_json(root.path());
    let task = find_task(&final_status, task_id);
    assert_eq!(task["state"], "done");
    assert!(done_task_path(root.path(), task_id).exists());
}

fn pass(root: &std::path::Path, task_id: &str, notes: &str) {
    let expected_state = support::task_state(root, task_id);
    gate_command(root)
        .args(["pass", task_id, "--root"])
        .arg(root)
        .args(["--agent", "test-agent"])
        .args(["--notes", notes])
        .args(["--expected-state", expected_state.as_str()])
        .assert()
        .success();
}

fn fail(root: &std::path::Path, task_id: &str, notes: &str) {
    let expected_state = support::task_state(root, task_id);
    gate_command(root)
        .args(["fail", task_id, "--root"])
        .arg(root)
        .args(["--agent", "test-agent"])
        .args(["--notes", notes])
        .args(["--expected-state", expected_state.as_str()])
        .assert()
        .success();
}

fn find_task<'a>(status: &'a serde_json::Value, task_id: &str) -> &'a serde_json::Value {
    status
        .as_array()
        .expect("status should be an array")
        .iter()
        .find(|entry| entry["taskId"] == task_id)
        .unwrap_or_else(|| panic!("missing task {task_id} in status output"))
}
