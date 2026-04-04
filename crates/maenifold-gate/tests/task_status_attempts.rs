mod support;

use std::fs;

use support::{new_pipeline_root, status_json, sync_gate};

fn write_custom_pipeline(root: &std::path::Path) {
    fs::write(
        root.join("pipeline.json"),
        r#"{
  "schemaVersion": 1,
  "id": "custom",
  "name": "Custom Pending Retry",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "swe", "fail": "pending" } },
    "swe": { "on": { "fail": "pending", "pass": "done" } },
    "done": { "terminal": true }
  }
}"#,
    )
    .expect("write custom pipeline");
}

fn find_task<'a>(status: &'a serde_json::Value, task_id: &str) -> &'a serde_json::Value {
    status
        .as_array()
        .expect("status should be an array")
        .iter()
        .find(|entry| entry["taskId"] == task_id)
        .unwrap_or_else(|| panic!("missing task {task_id} in status output"))
}

#[test]
fn fail_from_pending_dispatch_does_not_increment_attempts() {
    let root = new_pipeline_root();
    write_custom_pipeline(root.path());
    let task = root.path().join("todo").join("T-90.22.md");
    fs::write(
        &task,
        "# T-90.22 — Pending fail\n\n**Pipeline**: custom\n\n## Pending\n\n- Stage ID: pending\n- Record Kind: outcome\n- Agent: test-agent\n- Recorded At: 2025-01-01T00:00:00Z\n- Outcome: FAIL\n- Notes: dispatch rejected before work started\n",
    )
    .expect("write pending fail task");

    sync_gate(root.path());
    let status = status_json(root.path());
    let entry = find_task(&status, "T-90.22");
    assert_eq!(entry["state"], "pending");
    assert_eq!(entry["attempts"], 0);
    assert!(
        fs::read_to_string(&task)
            .expect("read task")
            .contains("FAIL")
    );
}

#[test]
fn fail_after_progress_back_to_pending_increments_attempts() {
    let root = new_pipeline_root();
    write_custom_pipeline(root.path());
    let task = root.path().join("todo").join("T-90.23.md");
    fs::write(
        &task,
        "# T-90.23 — Retry fail\n\n**Pipeline**: custom\n\n## Pending\n\n- Stage ID: pending\n- Record Kind: outcome\n- Agent: test-agent\n- Recorded At: 2025-01-01T00:00:00Z\n- Outcome: PASS\n- Notes: swe started work\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: test-agent\n- Recorded At: 2025-01-01T00:05:00Z\n- Outcome: FAIL\n- Notes: work cycle failed\n",
    )
    .expect("write retry fail task");

    sync_gate(root.path());
    let status = status_json(root.path());
    let entry = find_task(&status, "T-90.23");
    assert_eq!(entry["state"], "pending");
    assert_eq!(entry["attempts"], 1);
}

#[test]
fn missing_pending_bootstrap_allows_fail_then_pass_to_reach_red_team() {
    let root = new_pipeline_root();
    let task = root.path().join("todo").join("T-90.22.md");
    fs::write(
        &task,
        "# T-90.22 — Missing bootstrap record\n\n**Pipeline**: sdd\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: test-agent\n- Recorded At: 2025-01-01T00:00:00Z\n- Outcome: FAIL\n- Notes: first SWE attempt failed before pending bootstrap was recorded\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: test-agent\n- Recorded At: 2025-01-01T00:05:00Z\n- Outcome: PASS\n- Notes: retry succeeded\n",
    )
    .expect("write missing bootstrap task");

    sync_gate(root.path());
    let status = status_json(root.path());
    let entry = find_task(&status, "T-90.22");
    assert_eq!(entry["state"], "red-team");
    assert_eq!(entry["attempts"], 1);
}
