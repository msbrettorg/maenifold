use crate::error::{MaenifoldError, Result};
use rmcp::model::{
    AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use urlencoding::decode;

use crate::mcp::resources::{gate_history, gate_status, gate_task};

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
        return Ok(gate_status::gate_status(&root));
    }

    if let Some(task_id) = path.strip_prefix("task/") {
        return gate_task::gate_task_body(&root, decode_segment(task_id)?);
    }

    if let Some(task_id) = path.strip_prefix("history/") {
        return gate_history::gate_task_history(&root, decode_segment(task_id)?);
    }

    Err(MaenifoldError::InvalidResourceUri(uri.to_string()))
}

pub(super) fn parse_query_root(query: &str) -> Result<Option<PathBuf>> {
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

pub(super) fn resolve_gate_root(query_root: Option<PathBuf>) -> Result<PathBuf> {
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

pub(super) fn find_task_path(root: &Path, task_id: &str) -> Option<PathBuf> {
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

pub(super) fn decode_segment(value: &str) -> Result<String> {
    decode(value)
        .map(|decoded| decoded.into_owned())
        .map_err(|_| MaenifoldError::InvalidResourceUri(format!("invalid URI segment '{value}'")))
}

pub(super) fn parse_frontmatter(markdown: &str) -> Option<serde_json::Map<String, Value>> {
    let normalized = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let remaining = normalized.strip_prefix("---\n")?;
    let end = remaining.find("\n---")?;
    let yaml = &remaining[..end];
    let yaml_value = serde_yaml::from_str::<serde_yaml::Value>(yaml).ok()?;
    serde_json::to_value(yaml_value)
        .ok()
        .and_then(|value| value.as_object().cloned())
}

pub(super) fn parse_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(str::to_string)
}

pub(super) fn parse_stage_headings(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| line.strip_prefix("## ").map(str::trim))
        .map(slug_stage)
        .filter(|stage| !stage.is_empty())
        .collect()
}

pub(super) fn slug_stage(value: &str) -> String {
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

pub(super) fn task_id_from_path(path: &Path) -> Option<String> {
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
