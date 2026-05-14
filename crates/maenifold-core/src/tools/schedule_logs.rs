use super::context::{ToolContext, render, required_string};
use crate::error::{MaenifoldError, Result};
use crate::schedule::ScheduleStore;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAX_LOG_BYTES_PER_FILE: u64 = 64 * 1024;
const MAX_TOTAL_LOG_BYTES: usize = 256 * 1024;
const LOG_OUTPUT_TRUNCATED_MESSAGE: &str = "... truncated";

pub(super) fn get_logs(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let limit = optional_u64(payload, "limit")?.unwrap_or(10) as usize;
    let store = ScheduleStore::new(context.config.maenifold_root.join("schedules"));
    store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let logs = read_recent_logs(&store.task_dir(id).join("logs"), limit)?;
    Ok(render(context.output_format, json!(logs)))
}

pub(super) fn clear_history(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = ScheduleStore::new(context.config.maenifold_root.join("schedules"));
    store
        .read(id)
        .map_err(|_| MaenifoldError::InvalidArgument(format!("unknown scheduleId '{id}'")))?;
    let log_dir = store.task_dir(id).join("logs");
    let files = list_log_files(&log_dir)?;
    let deleted_files = files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    for path in files {
        fs::remove_file(&path)?;
    }
    let remaining = list_log_files(&log_dir)?;
    if !remaining.is_empty() {
        return Err(MaenifoldError::Message(format!(
            "failed to clear schedule history for '{}': {} log files remain",
            id,
            remaining.len()
        )));
    }
    Ok(render(
        context.output_format,
        json!({
            "action": "clear_history",
            "id": id,
            "deletedCount": deleted_files.len(),
            "deletedFiles": deleted_files,
        }),
    ))
}

fn list_log_files(log_dir: &Path) -> Result<Vec<PathBuf>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

fn read_recent_logs(log_dir: &Path, limit: usize) -> Result<Vec<Value>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(log_dir)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("log"))
        .collect::<Vec<_>>();
    entries.sort();
    entries.reverse();
    let mut logs = Vec::new();
    let mut total_bytes = 0usize;
    for path in entries.into_iter().take(limit) {
        if total_bytes >= MAX_TOTAL_LOG_BYTES {
            logs.push(json!({"fileName":"...","content":LOG_OUTPUT_TRUNCATED_MESSAGE}));
            break;
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| MaenifoldError::NonUtf8Path(path.clone()))?;
        let content = read_log_tail(&path)?;
        let remaining_bytes = MAX_TOTAL_LOG_BYTES - total_bytes;
        let output = if content.len() > remaining_bytes {
            total_bytes = MAX_TOTAL_LOG_BYTES;
            truncate_with_suffix(&content, remaining_bytes, LOG_OUTPUT_TRUNCATED_MESSAGE)
        } else {
            total_bytes += content.len();
            content
        };
        logs.push(json!({"fileName":file_name,"content":output}));
    }
    Ok(logs)
}

fn read_log_tail(path: &Path) -> Result<String> {
    let file_size = fs::metadata(path)?.len();
    let mut file = fs::File::open(path)?;
    if file_size > MAX_LOG_BYTES_PER_FILE {
        file.seek(SeekFrom::End(-(MAX_LOG_BYTES_PER_FILE as i64)))?;
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let content = String::from_utf8_lossy(&bytes).into_owned();
    if file_size <= MAX_LOG_BYTES_PER_FILE {
        return Ok(content);
    }

    Ok(match content.find('\n') {
        Some(position) => content[position + 1..].to_string(),
        None => content,
    })
}

fn truncate_with_suffix(content: &str, max_bytes: usize, suffix: &str) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    if max_bytes == 0 {
        return String::new();
    }
    if max_bytes <= suffix.len() {
        return suffix[..truncate_to_char_boundary(suffix, max_bytes)].to_string();
    }

    let prefix_end = truncate_to_char_boundary(content, max_bytes - suffix.len());
    let mut truncated = content[..prefix_end].to_string();
    truncated.push_str(suffix);
    truncated
}

fn truncate_to_char_boundary(content: &str, max_bytes: usize) -> usize {
    let mut end = max_bytes.min(content.len());
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn optional_u64(payload: &Value, key: &'static str) -> Result<Option<u64>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an integer")))
}
