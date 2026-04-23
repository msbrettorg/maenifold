use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use crate::mcp::resources::gate::{
    parse_frontmatter, parse_stage_headings, parse_title, task_id_from_path,
};

pub(super) fn gate_status(root: &Path) -> Value {
    let mut rows = Vec::new();
    for (location, dir_name) in [("todo", "todo"), ("done", "done")] {
        let dir = root.join(dir_name);
        if let Ok(read_dir) = fs::read_dir(&dir) {
            for entry in read_dir.filter_map(|item| item.ok()) {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                if let Some(task) = status_row_from_file(&path, location) {
                    rows.push(task);
                }
            }
        }
    }
    rows.sort_by(|a, b| {
        let left = a
            .get("taskId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let right = b
            .get("taskId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        left.cmp(&right)
    });
    Value::Array(rows)
}

fn status_row_from_file(path: &Path, location: &str) -> Option<Value> {
    let markdown = fs::read_to_string(path).ok()?;
    let task_id = task_id_from_path(path)?;
    let title = parse_title(&markdown).unwrap_or_else(|| task_id.clone());
    let frontmatter = parse_frontmatter(&markdown);
    let pipeline = frontmatter
        .as_ref()
        .and_then(|map| map.get("pipeline"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut state = frontmatter
        .as_ref()
        .and_then(|map| map.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("pending")
        .to_string();
    if location == "done" {
        state = "done".to_string();
    }

    let stages = parse_stage_headings(&markdown);

    Some(json!({
        "taskId": task_id,
        "canonicalTaskId": Value::Null,
        "title": title,
        "pipeline": pipeline,
        "state": state,
        "stages": stages,
        "attempts": 0,
        "location": location,
        "path": path,
        "issue": "",
    }))
}
