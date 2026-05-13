use std::fs;
use std::io::Write;

use serde_json::{Value, json};

use super::lifecycle_helpers::status_row;
use crate::error::GateError;
use crate::task_metadata::TaskMetadataParser;
use crate::task_repository::{TaskPathSet, TaskRepository};

pub(super) fn error_status_row(
    root: &std::path::Path,
    task: &TaskPathSet,
    error: GateError,
) -> Value {
    let mut title = format!("[error] {}", task.task_id);
    let mut pipeline = "unknown".to_string();
    let mut canonical_task_id = task.task_id.clone();

    if let Ok(path) = TaskRepository::canonical_task_markdown_path(root, task)
        && let Ok(text) = fs::read_to_string(&path)
    {
        let lines = text.lines().collect::<Vec<_>>();
        if let Ok(metadata) = TaskMetadataParser::parse(&lines) {
            title = metadata.title;
            pipeline = metadata.pipeline;
            if let Some(id) = metadata.id {
                canonical_task_id = id;
            }
        } else if let Ok(metadata) = TaskMetadataParser::parse_lenient(&lines) {
            title = metadata.title;
            pipeline = metadata.pipeline;
            if let Some(id) = metadata.id {
                canonical_task_id = id;
            }
        }
    }

    json!({
        "taskId": task.task_id,
        "canonicalTaskId": canonical_task_id,
        "title": title,
        "pipeline": pipeline,
        "state": "error",
        "stages": [],
        "attempts": 0,
        "location": task.location,
        "path": task.path.display().to_string(),
        "linked": false,
        "issue": format!("{error}"),
        "priority": serde_json::Value::Null,
        "sprint": serde_json::Value::Null,
        "lastUpdatedAt": serde_json::Value::Null,
    })
}

pub(super) fn status_summary_row(row: &Value) -> Value {
    json!({
        "taskId": row.get("taskId").cloned().unwrap_or(Value::Null),
        "state": row.get("state").cloned().unwrap_or(Value::Null),
        "location": row.get("location").cloned().unwrap_or(Value::Null),
        "attempts": row.get("attempts").cloned().unwrap_or(Value::Null),
        "linked": row.get("linked").cloned().unwrap_or(Value::Null),
        "sprint": row.get("sprint").cloned().unwrap_or(Value::Null),
    })
}

pub(super) fn write_gate_db_snapshot(
    root: &std::path::Path,
    gate_dir: &std::path::Path,
    tasks: &[TaskPathSet],
) -> Result<(), GateError> {
    let rows = tasks
        .iter()
        .map(|task| match status_row(root, task, None) {
            Ok(row) => row,
            Err(error) => error_status_row(root, task, error),
        })
        .collect::<Vec<_>>();
    let snapshot = json!({
        "schemaVersion": 1,
        "source": "maenifold.gate markdown read model",
        "tasks": rows,
    });
    let db_path = gate_dir.join("gate.db");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&db_path)
        .map_err(|error| {
            GateError::Io(std::io::Error::new(
                error.kind(),
                format!(
                    "failed to open gate.db read model '{}': {error}",
                    db_path.display()
                ),
            ))
        })?;
    file.write_all(serde_json::to_string_pretty(&snapshot)?.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
