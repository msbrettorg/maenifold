use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value as YamlValue;

use crate::error::Result;
use crate::tools::ToolContext;

pub(super) struct GateTaskStatus {
    pub(super) path: PathBuf,
    pub(super) effective_state: String,
    pub(super) terminal: bool,
}

pub(super) fn find_gate_task_status(
    context: &ToolContext,
    task_id: &str,
    persisted_target: Option<&str>,
) -> Result<Option<GateTaskStatus>> {
    if let Some(path) = persisted_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return resolve_registered_gate_task_status(&path);
    }

    for candidate_root in gate_root_candidates(context) {
        let done_dir = candidate_root.join("done");
        if let Some(path) = find_gate_task_in_dir(&done_dir, task_id)? {
            return Ok(Some(gate_task_status_from_path(path)?));
        }

        let todo_dir = candidate_root.join("todo");
        if let Some(path) = find_gate_task_in_dir(&todo_dir, task_id)? {
            return Ok(Some(gate_task_status_from_path(path)?));
        }
    }

    Ok(None)
}

fn resolve_registered_gate_task_status(path: &Path) -> Result<Option<GateTaskStatus>> {
    if path.is_file() {
        return Ok(Some(gate_task_status_from_path(path.to_path_buf())?));
    }

    let Some(file_name) = path.file_name() else {
        return Ok(None);
    };
    let Some(parent) = path.parent() else {
        return Ok(None);
    };

    if parent.file_name().and_then(|segment| segment.to_str()) == Some("todo") {
        let moved_path = parent
            .parent()
            .map(|root| root.join("done").join(file_name))
            .filter(|candidate| candidate.is_file());
        if let Some(moved_path) = moved_path {
            return Ok(Some(gate_task_status_from_path(moved_path)?));
        }
    }

    Ok(None)
}

fn gate_root_candidates(context: &ToolContext) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    collect_gate_roots(&mut candidates, &context.config.maenifold_root);
    if let Ok(current_dir) = std::env::current_dir() {
        collect_gate_roots(&mut candidates, &current_dir);
    }
    candidates
}

fn collect_gate_roots(candidates: &mut Vec<PathBuf>, start: &Path) {
    for ancestor in start.ancestors() {
        if ancestor.join("todo").is_dir()
            && ancestor.join("done").is_dir()
            && !candidates.iter().any(|existing| existing == ancestor)
        {
            candidates.push(ancestor.to_path_buf());
        }
    }
}

fn find_gate_task_in_dir(dir: &Path, task_id: &str) -> Result<Option<PathBuf>> {
    if !dir.is_dir() {
        return Ok(None);
    }

    let mut matches = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("md")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == task_id || stem.starts_with(&format!("{task_id}-")))
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches.into_iter().next())
}

fn read_gate_task_state(path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(path)?;
    let yaml = extract_yaml_frontmatter(&content);
    let Some(yaml) = yaml else {
        return Ok(None);
    };
    let parsed = serde_yaml::from_str::<serde_yaml::Mapping>(&yaml)?;
    Ok(parsed
        .get(&YamlValue::String("state".to_string()))
        .and_then(YamlValue::as_str)
        .map(str::to_string))
}

fn gate_task_status_from_path(path: PathBuf) -> Result<GateTaskStatus> {
    let effective_state = if path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|segment| segment.to_str())
        .is_some_and(|segment| segment == "done")
    {
        "done".to_string()
    } else {
        read_gate_task_state(&path)?.unwrap_or_else(|| "pending".to_string())
    };
    Ok(GateTaskStatus {
        path,
        terminal: effective_state.eq_ignore_ascii_case("done"),
        effective_state,
    })
}

fn extract_yaml_frontmatter(content: &str) -> Option<String> {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let remainder = normalized.strip_prefix("---\n")?;
    let end = remainder.find("\n---")?;
    Some(remainder[..end].to_string())
}
