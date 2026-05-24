use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::schedule::ScheduleStore;
use rmcp::model::{
    AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(test)]
#[path = "schedule_consistency_helpers.rs"]
mod schedule_consistency_helpers;

#[cfg(test)]
#[path = "schedule_consistency_tests.rs"]
mod schedule_consistency_tests;

const JOBS_URI: &str = "schedule://jobs";
const JOB_TEMPLATE_URI: &str = "schedule://job/{id}";
const LOGS_TEMPLATE_URI: &str = "schedule://logs/{id}";
const JOB_URI_PREFIX: &str = "schedule://job/";
const LOGS_URI_PREFIX: &str = "schedule://logs/";
const MAX_LOG_BYTES_PER_FILE: u64 = 64 * 1024;
const MAX_TOTAL_LOG_BYTES: usize = 256 * 1024;
const LOG_OUTPUT_TRUNCATED_MESSAGE: &str = "... truncated";

pub(super) fn list_resources() -> Vec<Resource> {
    vec![make_resource(
        JOBS_URI,
        "Scheduled Jobs",
        "List of all scheduled jobs.",
    )]
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        make_template(
            JOB_TEMPLATE_URI,
            "Scheduled Job",
            "Details for a scheduled job by id.",
        ),
        make_template(
            LOGS_TEMPLATE_URI,
            "Scheduled Job Logs",
            "Recent logs for a scheduled job by id.",
        ),
    ]
}

pub(super) fn is_schedule_uri(uri: &str) -> bool {
    uri.starts_with("schedule://")
}

#[cfg(test)]
pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    handle_read_resource_with_config(uri, &Config::current())
}

pub(super) fn handle_read_resource_with_config(
    uri: &str,
    config: &Config,
) -> Result<ReadResourceResult> {
    let payload = match uri {
        JOBS_URI => list_jobs_payload(config)?,
        _ if uri.starts_with(JOB_URI_PREFIX) => {
            let id = parse_id(uri, JOB_URI_PREFIX, JOB_TEMPLATE_URI)?;
            read_job_payload(config, &id)?
        }
        _ if uri.starts_with(LOGS_URI_PREFIX) => {
            let id = parse_id(uri, LOGS_URI_PREFIX, LOGS_TEMPLATE_URI)?;
            read_logs_payload(config, &id)?
        }
        _ => {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected '{JOBS_URI}', '{JOB_TEMPLATE_URI}', or '{LOGS_TEMPLATE_URI}'"
            )));
        }
    };

    let text = serde_json::to_string_pretty(&payload)?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn list_jobs_payload(config: &Config) -> Result<Value> {
    let jobs = schedule_store(config)
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(serde_json::to_value(jobs)?)
}

fn read_job_payload(config: &Config, id: &str) -> Result<Value> {
    let store = schedule_store(config);
    if !store.exists(id) {
        return Err(schedule_not_found(id));
    }

    let job = store
        .read(id)
        .map_err(|_| MaenifoldError::Message(format!("failed to read scheduled task '{id}'")))?;
    Ok(serde_json::to_value(job)?)
}

fn read_logs_payload(config: &Config, id: &str) -> Result<Value> {
    let store = schedule_store(config);
    if !store.exists(id) {
        return Err(schedule_not_found(id));
    }

    let logs = read_recent_logs(&store.task_dir(id).join("logs"), 10)?;
    Ok(json!(logs))
}

fn schedule_store(config: &Config) -> ScheduleStore {
    ScheduleStore::new(config.maenifold_root.join("schedules"))
}

fn schedule_not_found(id: &str) -> MaenifoldError {
    MaenifoldError::AssetNotFound("scheduled-task".to_string(), id.to_string())
}

fn parse_id(uri: &str, prefix: &str, template: &str) -> Result<String> {
    let encoded = uri.strip_prefix(prefix).ok_or_else(|| {
        MaenifoldError::InvalidResourceUri(format!("{uri}. Expected '{template}'"))
    })?;
    let decoded = urlencoding::decode(encoded).map_err(|error| {
        MaenifoldError::InvalidResourceUri(format!(
            "Failed to decode schedule id '{encoded}': {error}"
        ))
    })?;
    let id = decoded.trim();
    if id.is_empty() || id.contains('/') {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{template}' with a non-empty id"
        )));
    }
    Ok(id.to_string())
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
            logs.push(json!({"fileName": "...", "content": LOG_OUTPUT_TRUNCATED_MESSAGE}));
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
        logs.push(log_entry(&path, file_name, output));
    }

    Ok(logs)
}

fn log_entry(path: &Path, file_name: &str, content: String) -> Value {
    let mut entry = serde_json::Map::from_iter([
        ("fileName".to_string(), json!(file_name)),
        ("content".to_string(), json!(content)),
    ]);
    if let Ok(Value::Object(sidecar)) = fs::read_to_string(path.with_extension("json"))
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(std::io::Error::other))
    {
        for key in [
            "status",
            "duration_seconds",
            "mcp_action",
            "mcp_parameters",
            "mcp_result",
        ] {
            if let Some(value) = sidecar.get(key) {
                entry.insert(key.to_string(), value.clone());
            }
        }
    }
    Value::Object(entry)
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

fn make_resource(uri: &str, name: &str, description: &str) -> Resource {
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    resource.title = Some(name.to_string());
    resource.description = Some(description.to_string());
    resource
}

fn make_template(uri_template: &str, name: &str, description: &str) -> ResourceTemplate {
    let mut template = RawResourceTemplate::new(uri_template, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    template.title = Some(name.to_string());
    template.description = Some(description.to_string());
    template
}
