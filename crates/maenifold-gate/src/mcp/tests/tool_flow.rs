use std::fs;

use serde_json::json;

use super::super::call_gate_tool;
use super::support::{new_custom_pipeline_root, task_markdown};

#[test]
fn mcp_status_tool_returns_task_array() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();
    call_gate_tool(
        "sync",
        &json!({ "root": root.path(), "rebuild": true }),
        Some(&root_path),
    )
    .expect("sync tool should succeed");
    let output = call_gate_tool("status", &json!({ "root": root.path() }), Some(&root_path))
        .expect("status tool should succeed");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
    let tasks = value.as_array().expect("status should return an array");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["taskId"], "T-9.1");
    assert_eq!(tasks[0]["state"], "pending");
}

#[test]
fn mcp_status_tool_supports_ready_and_state_filters() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();
    fs::write(
        root.path().join("todo").join("T-9.2.md"),
        task_markdown("T-9.2", "custom"),
    )
    .expect("write second task");
    call_gate_tool(
        "sync",
        &json!({ "root": root.path(), "rebuild": true }),
        Some(&root_path),
    )
    .expect("sync tool should succeed");
    call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.2",
            "agent": "swe",
            "notes": "advanced"
        }),
        Some(&root_path),
    )
    .expect("pass tool should succeed");

    let ready_output = call_gate_tool(
        "status",
        &json!({ "root": root.path(), "ready": true }),
        Some(&root_path),
    )
    .expect("ready status should succeed");
    let ready_value: serde_json::Value =
        serde_json::from_str(&ready_output).expect("valid ready JSON output");
    let ready_tasks = ready_value
        .as_array()
        .expect("ready status should return an array");
    assert_eq!(ready_tasks.len(), 1);
    assert_eq!(ready_tasks[0]["taskId"], "T-9.1");

    let swe_output = call_gate_tool(
        "status",
        &json!({ "root": root.path(), "state": "swe" }),
        Some(&root_path),
    )
    .expect("state status should succeed");
    let swe_value: serde_json::Value =
        serde_json::from_str(&swe_output).expect("valid state JSON output");
    let swe_tasks = swe_value
        .as_array()
        .expect("state status should return an array");
    assert_eq!(swe_tasks.len(), 1);
    assert_eq!(swe_tasks[0]["taskId"], "T-9.2");
}

#[test]
fn mcp_status_excludes_done_by_default_and_restores_them_with_include_done() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();
    fs::write(
        root.path().join("todo").join("T-9.2.md"),
        task_markdown("T-9.2", "custom"),
    )
    .expect("write second task");
    call_gate_tool(
        "sync",
        &json!({ "root": root.path(), "rebuild": true }),
        Some(&root_path),
    )
    .expect("sync tool should succeed");
    call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.2",
            "agent": "swe",
            "notes": "advanced to swe"
        }),
        Some(&root_path),
    )
    .expect("advance to swe");
    call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.2",
            "agent": "red-team",
            "notes": "advanced to red-team"
        }),
        Some(&root_path),
    )
    .expect("advance to red-team");
    call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.2",
            "agent": "red-team",
            "notes": "completed task"
        }),
        Some(&root_path),
    )
    .expect("complete task");

    let default_output =
        call_gate_tool("status", &json!({ "root": root.path() }), Some(&root_path))
            .expect("default status should succeed");
    let default_value: serde_json::Value =
        serde_json::from_str(&default_output).expect("valid default status json");
    let default_tasks = default_value
        .as_array()
        .expect("default status should return an array");
    assert_eq!(default_tasks.len(), 1);
    assert_eq!(default_tasks[0]["taskId"], "T-9.1");

    let all_output = call_gate_tool(
        "status",
        &json!({ "root": root.path(), "includeDone": true }),
        Some(&root_path),
    )
    .expect("includeDone status should succeed");
    let all_value: serde_json::Value =
        serde_json::from_str(&all_output).expect("valid includeDone status json");
    let all_tasks = all_value
        .as_array()
        .expect("includeDone status should return an array");
    assert_eq!(all_tasks.len(), 2);
    assert!(all_tasks.iter().any(|task| task["taskId"] == "T-9.2"));

    let done_output = call_gate_tool(
        "status",
        &json!({ "root": root.path(), "state": "done" }),
        Some(&root_path),
    )
    .expect("done state status should succeed");
    let done_value: serde_json::Value =
        serde_json::from_str(&done_output).expect("valid done status json");
    let done_tasks = done_value
        .as_array()
        .expect("done state status should return an array");
    assert_eq!(done_tasks.len(), 1);
    assert_eq!(done_tasks[0]["taskId"], "T-9.2");
}

#[test]
fn mcp_pass_tool_advances_bound_task() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();
    let output = call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.1",
            "agent": "swe",
            "notes": "swe completed"
        }),
        Some(&root_path),
    )
    .expect("pass tool should succeed");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
    assert_eq!(value["taskId"], "T-9.1");
    assert_eq!(value["state"], "swe");
}

#[test]
fn mcp_history_tool_returns_stage_journey() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();

    call_gate_tool(
        "append",
        &json!({
            "root": root.path(),
            "taskId": "T-9.1",
            "agent": "swe",
            "notes": "started implementation",
            "model": "claude-sonnet-4.5"
        }),
        Some(&root_path),
    )
    .expect("append tool should succeed");
    call_gate_tool(
        "pass",
        &json!({
            "root": root.path(),
            "taskId": "T-9.1",
            "agent": "swe",
            "notes": "completed implementation"
        }),
        Some(&root_path),
    )
    .expect("pass tool should succeed");
    call_gate_tool(
        "fail",
        &json!({
            "root": root.path(),
            "taskId": "T-9.1",
            "agent": "red-team",
            "notes": "requested rework"
        }),
        Some(&root_path),
    )
    .expect("fail tool should succeed");

    let output = call_gate_tool(
        "history",
        &json!({ "root": root.path(), "taskId": "T-9.1" }),
        Some(&root_path),
    )
    .expect("history tool should succeed");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
    let history = value.as_array().expect("history should return an array");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0]["stage"], "pending");
    assert_eq!(history[0]["event"], "append");
    assert_eq!(history[0]["agent"], "swe");
    assert_eq!(history[0]["model"], "claude-sonnet-4.5");
    assert_eq!(history[0]["notes"], "started implementation");
    assert!(history[0]["timestamp"].as_str().is_some());
    assert_eq!(history[1]["event"], "PASS");
    assert_eq!(history[1]["notes"], "completed implementation");
    assert_eq!(history[2]["stage"], "swe");
    assert_eq!(history[2]["event"], "FAIL");
    assert_eq!(history[2]["agent"], "red-team");
    assert_eq!(history[2]["notes"], "requested rework");
}
