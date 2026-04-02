mod support;

use support::{
    authoritative_digest, done_task_path, gate_command, status_json, task_path, write_bound_task,
};

#[test]
fn sdd_e2e_moves_task_to_done() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.1";
    let todo_path = write_bound_task(root.path(), task_id, "sdd", authoritative_digest("sdd"));

    pass(root.path(), task_id, "swe completed");
    pass(root.path(), task_id, "red approved");
    pass(root.path(), task_id, "blue verified");

    assert!(!todo_path.exists());
    assert!(done_task_path(root.path(), task_id).exists());

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["status"], "done");
    assert_eq!(task["location"], "done");
}

#[test]
fn tdd_e2e_moves_task_to_done() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.2";
    let todo_path = write_bound_task(root.path(), task_id, "tdd", authoritative_digest("tdd"));

    pass(root.path(), task_id, "swe completed");
    pass(root.path(), task_id, "red approved");
    pass(root.path(), task_id, "blue verified");

    assert!(!todo_path.exists());
    assert!(done_task_path(root.path(), task_id).exists());

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "tdd");
    assert_eq!(task["status"], "done");
    assert_eq!(task["location"], "done");
}

#[test]
fn gsd_e2e_fail_from_verify_returns_to_execute_then_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.3";
    write_bound_task(root.path(), task_id, "gsd", authoritative_digest("gsd"));

    pass(root.path(), task_id, "research done");
    pass(root.path(), task_id, "plan done");
    pass(root.path(), task_id, "execute done");
    fail(root.path(), task_id, "verify failed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "gsd");
    assert_eq!(task["status"], "execute");
    assert_eq!(task["location"], "todo");
    assert_eq!(task["attempts"], 0);
    assert!(task_path(root.path(), task_id).exists());

    pass(root.path(), task_id, "execute remediation done");
    pass(root.path(), task_id, "verify passed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["status"], "done");
    assert_eq!(task["location"], "done");
    assert!(done_task_path(root.path(), task_id).exists());
}

#[test]
fn lightweight_e2e_fail_from_blue_returns_to_pending_then_completes() {
    let root = support::new_pipeline_root();
    let task_id = "T-2.4";
    write_bound_task(
        root.path(),
        task_id,
        "lightweight",
        authoritative_digest("lightweight"),
    );

    pass(root.path(), task_id, "swe completed");
    fail(root.path(), task_id, "blue failed");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["pipeline"], "lightweight");
    assert_eq!(task["status"], "pending");
    assert_eq!(task["location"], "todo");
    assert_eq!(task["attempts"], 1);
    assert!(task_path(root.path(), task_id).exists());

    pass(root.path(), task_id, "swe remediation completed");
    pass(root.path(), task_id, "blue verified");

    let status = status_json(root.path());
    let task = find_task(&status, task_id);
    assert_eq!(task["status"], "done");
    assert_eq!(task["location"], "done");
    assert!(done_task_path(root.path(), task_id).exists());
}

fn pass(root: &std::path::Path, task_id: &str, notes: &str) {
    gate_command()
        .args(["pass", task_id, "--root"])
        .arg(root)
        .args(["--notes", notes])
        .assert()
        .success();
}

fn fail(root: &std::path::Path, task_id: &str, notes: &str) {
    gate_command()
        .args(["fail", task_id, "--root"])
        .arg(root)
        .args(["--notes", notes])
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
