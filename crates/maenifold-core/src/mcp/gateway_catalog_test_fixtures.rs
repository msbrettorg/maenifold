use crate::mcp::gateway_gate;
use crate::util::paths::slugify_title;
use rmcp::model::CallToolResult;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub(super) async fn invoke_create(
    root: &Path,
    task_id: &str,
    title: &str,
    pipeline: &str,
    extra: serde_json::Value,
) -> CallToolResult {
    let mut parameters = serde_json::Map::from_iter([
        ("root".to_string(), json!(root.display().to_string())),
        ("taskId".to_string(), json!(task_id)),
        ("title".to_string(), json!(title)),
        ("pipeline".to_string(), json!(pipeline)),
    ]);
    if let Some(extra) = extra.as_object() {
        parameters.extend(extra.clone());
    }
    gateway_gate::invoke_gate(
        "gate_Create".to_string(),
        serde_json::Value::Object(parameters),
    )
    .await
    .expect("gate create should return CallToolResult")
}

pub(super) fn gate_root() -> TempDir {
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("todo")).expect("todo");
    fs::create_dir_all(root.path().join("done")).expect("done");
    fs::create_dir_all(root.path().join("assets/gates")).expect("assets/gates");
    root
}

pub(super) fn write_existing_task(
    root: &Path,
    location: &str,
    task_id: &str,
    title: &str,
    pipeline: &str,
) {
    let path = root
        .join(location)
        .join(format!("{task_id}-{}.md", slugify_title(title)));
    fs::write(
        path,
        markdown(
            task_id,
            title,
            pipeline,
            Some("P2"),
            Some(99),
            None,
            "## Problem\n\nExisting task.\n",
        ),
    )
    .expect("write existing task");
}

pub(super) fn write_custom_pipeline(root: &Path, pipeline: &str) {
    let definition = json!({
        "schemaVersion": 1,
        "id": pipeline,
        "name": "Custom Create Pipeline",
        "initial": "pending",
        "states": {
            "pending": { "on": { "pass": "done" } },
            "done": { "terminal": true }
        }
    });
    fs::write(
        root.join("assets/gates").join(format!("{pipeline}.json")),
        serde_json::to_string_pretty(&definition).expect("serialize definition"),
    )
    .expect("write custom pipeline");
}

pub(super) fn markdown(
    task_id: &str,
    title: &str,
    pipeline: &str,
    priority: Option<&str>,
    sprint: Option<u64>,
    parent: Option<&str>,
    body: &str,
) -> String {
    let mut frontmatter = vec![
        "---".to_string(),
        format!("id: {task_id}"),
        format!("title: {title:?}"),
        format!("pipeline: {pipeline}"),
    ];
    if let Some(priority) = priority {
        frontmatter.push(format!("priority: {priority}"));
    }
    if let Some(sprint) = sprint {
        frontmatter.push(format!("sprint: {sprint}"));
    }
    if let Some(parent) = parent {
        frontmatter.push(format!("parent: {parent}"));
    }
    frontmatter.push("---".to_string());
    format!("{}\n\n{body}", frontmatter.join("\n"))
}

pub(super) fn frontmatter(markdown: &str) -> serde_json::Value {
    let mut lines = markdown.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "task should begin with YAML frontmatter"
    );
    let mut yaml = Vec::new();
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
        yaml.push(line);
    }
    serde_yml::from_str::<serde_json::Value>(&yaml.join("\n")).expect("valid yaml frontmatter")
}

pub(super) fn parsed_json(result: &CallToolResult) -> serde_json::Value {
    serde_json::from_str(&text(result)).expect("gateway response should be valid JSON")
}

pub(super) fn text(result: &CallToolResult) -> String {
    result.content[0]
        .raw
        .as_text()
        .expect("text content")
        .text
        .clone()
}
