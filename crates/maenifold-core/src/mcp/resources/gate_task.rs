use crate::error::{MaenifoldError, Result};
use crate::mcp::resources::gate::find_task_path;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn gate_task_body(root: &Path, task_id: String) -> Result<Value> {
    let path = find_task_path(root, &task_id)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("task '{task_id}' not found")))?;
    let markdown = fs::read_to_string(&path)?;
    Ok(json!({
        "taskId": task_id,
        "path": path,
        "markdown": markdown,
    }))
}
