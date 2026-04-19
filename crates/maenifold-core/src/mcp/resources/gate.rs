use crate::error::{MaenifoldError, Result};
use rmcp::model::{
    AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use urlencoding::decode;

pub(super) fn list_resources() -> Vec<Resource> {
    vec![RawResource::new("gate://status", "Gate status")
        .with_title("Gate status")
        .with_description("Current gate task status payload")
        .with_mime_type("application/json")
        .optional_annotate(None)]
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        RawResourceTemplate::new("gate://task/{id}", "Gate task")
            .with_title("Gate task body")
            .with_description("Gate task markdown body for task id")
            .with_mime_type("application/json")
            .optional_annotate(None),
        RawResourceTemplate::new("gate://history/{id}", "Gate task history")
            .with_title("Gate task history")
            .with_description("Gate task stage history for task id")
            .with_mime_type("application/json")
            .optional_annotate(None),
    ]
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let payload = read_gate_resource(uri)?;
    let text = serde_json::to_string_pretty(&payload)?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        text,
        uri.to_string(),
    )
    .with_mime_type("application/json")]))
}

fn read_gate_resource(uri: &str) -> Result<Value> {
    let raw = uri
        .strip_prefix("gate://")
        .ok_or_else(|| MaenifoldError::InvalidResourceUri(uri.to_string()))?;
    let (path, query) = raw.split_once('?').map_or((raw, ""), |(p, q)| (p, q));
    let root = resolve_gate_root(parse_query_root(query)?)?;

    if path == "status" {
        return Ok(gate_status(&root));
    }

    if let Some(task_id) = path.strip_prefix("task/") {
        return gate_task_body(&root, decode_segment(task_id)?);
    }

    if let Some(task_id) = path.strip_prefix("history/") {
        return gate_task_history(&root, decode_segment(task_id)?);
    }

    Err(MaenifoldError::InvalidResourceUri(uri.to_string()))
}

fn parse_query_root(query: &str) -> Result<Option<PathBuf>> {
    for pair in query.split('&').filter(|segment| !segment.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key != "root" {
            continue;
        }
        let decoded = decode(value).map_err(|_| {
            MaenifoldError::InvalidResourceUri(format!("invalid root query: {query}"))
        })?;
        return Ok(Some(PathBuf::from(decoded.into_owned())));
    }
    Ok(None)
}

fn resolve_gate_root(query_root: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(root) = query_root {
        if root.join("todo").is_dir() && root.join("done").is_dir() {
            return Ok(root);
        }
        return Err(MaenifoldError::InvalidArgument(format!(
            "root '{}' is not a gate workspace (todo/ and done/ required)",
            root.display()
        )));
    }

    let cwd = std::env::current_dir()?;
    for ancestor in cwd.ancestors() {
        if ancestor.join("todo").is_dir() && ancestor.join("done").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Err(MaenifoldError::InvalidArgument(
        "could not resolve gate root from current directory".to_string(),
    ))
}

fn gate_status(root: &Path) -> Value {
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

fn gate_task_body(root: &Path, task_id: String) -> Result<Value> {
    let path = find_task_path(root, &task_id)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("task '{task_id}' not found")))?;
    let markdown = fs::read_to_string(&path)?;
    Ok(json!({
        "taskId": task_id,
        "path": path,
        "markdown": markdown,
    }))
}

fn gate_task_history(root: &Path, task_id: String) -> Result<Value> {
    let path = find_task_path(root, &task_id)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("task '{task_id}' not found")))?;
    let markdown = fs::read_to_string(path)?;
    Ok(Value::Array(parse_history_records(&markdown)))
}

fn find_task_path(root: &Path, task_id: &str) -> Option<PathBuf> {
    for dir_name in ["todo", "done"] {
        let dir = root.join(dir_name);
        let Ok(read_dir) = fs::read_dir(dir) else {
            continue;
        };
        for entry in read_dir.filter_map(|item| item.ok()) {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str())?;
            if stem == task_id || stem.starts_with(&format!("{task_id}-")) {
                return Some(path);
            }
        }
    }
    None
}

fn decode_segment(value: &str) -> Result<String> {
    decode(value)
        .map(|decoded| decoded.into_owned())
        .map_err(|_| MaenifoldError::InvalidResourceUri(format!("invalid URI segment '{value}'")))
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

fn parse_frontmatter(markdown: &str) -> Option<serde_json::Map<String, Value>> {
    let normalized = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let remaining = normalized.strip_prefix("---\n")?;
    let end = remaining.find("\n---")?;
    let yaml = &remaining[..end];
    let yaml_value = serde_yaml::from_str::<serde_yaml::Value>(yaml).ok()?;
    serde_json::to_value(yaml_value)
        .ok()
        .and_then(|value| value.as_object().cloned())
}

fn parse_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(str::to_string)
}

fn parse_stage_headings(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| line.strip_prefix("## ").map(str::trim))
        .map(slug_stage)
        .filter(|stage| !stage.is_empty())
        .collect()
}

fn slug_stage(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn task_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if !stem.starts_with("T-") {
        return None;
    }
    let mut id = String::from("T-");
    for ch in stem["T-".len()..].chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' {
            id.push(ch);
            continue;
        }
        break;
    }
    if id.len() > 2 {
        Some(id)
    } else {
        None
    }
}

fn parse_history_records(markdown: &str) -> Vec<Value> {
    let mut records = Vec::new();
    let mut current_stage = String::new();
    let mut stage_id = String::new();
    let mut record_kind: Option<String> = None;
    let mut outcome: Option<String> = None;
    let mut agent: Option<String> = None;
    let mut model: Option<String> = None;
    let mut notes: Option<String> = None;
    let mut timestamp: Option<String> = None;

    let flush = |records: &mut Vec<Value>,
                 stage_id: &str,
                 current_stage: &str,
                 record_kind: &Option<String>,
                 outcome: &Option<String>,
                 agent: &Option<String>,
                 model: &Option<String>,
                 notes: &Option<String>,
                 timestamp: &Option<String>| {
        if record_kind.is_none()
            && outcome.is_none()
            && agent.is_none()
            && model.is_none()
            && notes.is_none()
            && timestamp.is_none()
        {
            return;
        }
        let stage = if !stage_id.is_empty() {
            stage_id.to_string()
        } else {
            slug_stage(current_stage)
        };
        let event = outcome
            .as_ref()
            .map(|value| value.to_uppercase())
            .or_else(|| record_kind.clone())
            .unwrap_or_else(|| "append".to_string());
        records.push(json!({
            "stage": stage,
            "event": event,
            "agent": agent,
            "model": model,
            "notes": notes,
            "timestamp": timestamp,
        }));
    };

    for line in markdown.lines() {
        if let Some(stage) = line.strip_prefix("## ").map(str::trim) {
            flush(
                &mut records,
                &stage_id,
                &current_stage,
                &record_kind,
                &outcome,
                &agent,
                &model,
                &notes,
                &timestamp,
            );
            current_stage = stage.to_string();
            stage_id.clear();
            record_kind = None;
            outcome = None;
            agent = None;
            model = None;
            notes = None;
            timestamp = None;
            continue;
        }
        let Some(payload) = line.strip_prefix("- ").map(str::trim) else {
            continue;
        };
        let Some((raw_key, raw_value)) = payload.split_once(':') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        match key.as_str() {
            "stage id" => stage_id = value.to_string(),
            "record kind" => record_kind = Some(value.to_string()),
            "outcome" => outcome = Some(value.to_string()),
            "agent" => agent = Some(value.to_string()),
            "model" => model = Some(value.to_string()),
            "notes" => notes = Some(value.to_string()),
            "recorded at" => timestamp = Some(value.to_string()),
            _ => {}
        }
    }

    flush(
        &mut records,
        &stage_id,
        &current_stage,
        &record_kind,
        &outcome,
        &agent,
        &model,
        &notes,
        &timestamp,
    );
    records
}
