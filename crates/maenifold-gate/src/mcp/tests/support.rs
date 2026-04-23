use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;

use super::super::{GateMcpHandler, call_gate_tool, optional_path};

pub(super) fn new_custom_pipeline_root() -> TempDir {
    new_custom_pipeline_root_with_task("T-9.1")
}

pub(super) fn new_custom_pipeline_root_with_task(task_id: &str) -> TempDir {
    let temp = tempfile::tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    fs::create_dir_all(root.join("done")).expect("create done");

    let pipeline_json = json!({
        "schemaVersion": 1,
        "id": "custom",
        "name": "Custom",
        "initial": "pending",
        "states": {
            "pending": { "on": { "pass": "swe" } },
            "swe": { "on": { "fail": "pending", "pass": "red-team" } },
            "red-team": { "on": { "fail": "pending", "pass": "done" } },
            "done": { "terminal": true }
        }
    });
    let pipeline_text =
        serde_json::to_string_pretty(&pipeline_json).expect("serialize pipeline json");
    fs::write(root.join("pipeline.json"), &pipeline_text).expect("write pipeline");

    fs::write(
        root.join("todo").join(format!("{task_id}.md")),
        task_markdown(task_id, "custom"),
    )
    .expect("write task");
    temp
}

pub(super) fn invoke_handler_tool(
    handler: &GateMcpHandler,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<String, String> {
    let bound_root = handler
        .bound_root
        .lock()
        .map_err(|err| err.to_string())?
        .clone();
    let explicit_root = optional_path(&arguments, "root").map_err(|err| err.to_string())?;
    let default_root = explicit_root
        .as_ref()
        .or(bound_root.as_ref())
        .or(handler.default_root.as_ref());
    match call_gate_tool(tool_name, &arguments, default_root) {
        Ok(output) => {
            if let Some(root) = explicit_root.as_ref() {
                let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
                handler
                    .known_roots
                    .register(canonical.clone())
                    .map_err(|err| err.to_string())?;
                *handler.bound_root.lock().map_err(|err| err.to_string())? = Some(canonical);
            }
            Ok(output)
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(super) fn single_task_id(output: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(output).expect("valid JSON output");
    let tasks = value.as_array().expect("status should return an array");
    assert_eq!(tasks.len(), 1, "expected exactly one task in output");
    tasks[0]["taskId"]
        .as_str()
        .expect("taskId should be a string")
        .to_string()
}

pub(super) fn task_markdown(task_id: &str, pipeline: &str) -> String {
    format!(
        "# {task_id} — MCP Test\n\n**Pipeline**: {pipeline}\n\n## Acceptance Criteria\n\n1. tool call succeeds\n"
    )
}

#[allow(dead_code)]
pub(super) fn _assert_exists(path: &Path) {
    assert!(path.exists(), "missing path {}", path.display());
}
