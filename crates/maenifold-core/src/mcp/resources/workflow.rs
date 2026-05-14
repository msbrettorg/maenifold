use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    extract_h2_sections, file_exists, get_memory_files, read_markdown, uri_to_path,
};
use crate::workflow::{
    WorkflowStepTimings, frontmatter_usize, read_step_timings, read_workflow_queue,
};
use rmcp::model::{
    AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents, ResourceTemplate,
};
use serde::Serialize;
use serde_yml::Value as YamlValue;
use std::time::SystemTime;

pub const WORKFLOW_SESSIONS_URI: &str = "workflow://sessions";
pub const WORKFLOW_SESSION_TEMPLATE_URI: &str = "workflow://session/{id}";
const WORKFLOW_SESSION_URI_PREFIX: &str = "workflow://session/";
const WORKFLOW_MEMORY_FOLDER_URI: &str = "memory://thinking/workflow";

#[derive(Debug, Serialize)]
struct WorkflowSessionsPayload {
    sessions: Vec<WorkflowSessionSummary>,
    errors: Vec<WorkflowSessionReadError>,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct WorkflowSessionReadError {
    uri: String,
    error: String,
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowSessionSummary {
    id: String,
    uri: String,
    status: String,
    queue: Vec<String>,
    workflow_id: Option<String>,
    current_workflow_index: usize,
    current_step: usize,
    total_steps: usize,
    started_at: Option<String>,
    updated_at: String,
    step_timings: WorkflowStepTimings,
}

#[derive(Debug, Serialize)]
struct WorkflowSessionDetail {
    #[serde(flatten)]
    session: WorkflowSessionSummary,
    history: Vec<WorkflowHistoryEntry>,
}

#[derive(Debug, Serialize)]
struct WorkflowHistoryEntry {
    heading: String,
    body: String,
}

pub(super) fn list_resources() -> Vec<Resource> {
    vec![make_resource(
        WORKFLOW_SESSIONS_URI,
        "Workflow Sessions",
        "List of workflow sessions available in memory.",
    )]
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        rmcp::model::RawResourceTemplate::new(
            WORKFLOW_SESSION_TEMPLATE_URI,
            "Workflow Session Detail",
        )
        .with_title("Workflow Session Detail")
        .with_description("Detailed workflow session payload for a specific session id.")
        .with_mime_type("application/json")
        .optional_annotate(None),
    ]
}

pub(super) fn is_workflow_uri(uri: &str) -> bool {
    uri.starts_with("workflow://")
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let payload = match uri {
        WORKFLOW_SESSIONS_URI => serde_json::to_string_pretty(&read_workflow_sessions()?)?,
        _ if uri.starts_with(WORKFLOW_SESSION_URI_PREFIX) => {
            serde_json::to_string_pretty(&read_workflow_session_detail(uri)?)?
        }
        _ => {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected '{WORKFLOW_SESSIONS_URI}' or '{WORKFLOW_SESSION_TEMPLATE_URI}'"
            )));
        }
    };

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(payload, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn read_workflow_sessions() -> Result<WorkflowSessionsPayload> {
    let config = Config::current();
    let mut sessions = Vec::new();
    let mut errors = Vec::new();

    for uri in get_memory_files(&config, Some(WORKFLOW_MEMORY_FOLDER_URI))? {
        match read_session_summary(&config, &uri) {
            Ok(session) => sessions.push(session),
            Err(error) => errors.push(WorkflowSessionReadError {
                uri,
                error: error.to_string(),
            }),
        }
    }

    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    errors.sort_by(|left, right| left.uri.cmp(&right.uri));

    Ok(WorkflowSessionsPayload {
        sessions,
        errors,
        generated_at: current_iso_timestamp(),
    })
}

fn read_workflow_session_detail(resource_uri: &str) -> Result<WorkflowSessionDetail> {
    let id = parse_session_id_from_resource_uri(resource_uri)?;
    let config = Config::current();
    let memory_uri = crate::sessions::workflow_session_uri(&id);
    if !file_exists(&memory_uri, &config)? {
        return Err(MaenifoldError::AssetNotFound(
            "workflow-session".to_string(),
            id,
        ));
    }

    let summary = read_session_summary(&config, &memory_uri)?;
    let (_, content, _) = read_markdown(&memory_uri, &config)?;
    let history = extract_h2_sections(&content)
        .into_iter()
        .map(|(heading, body)| WorkflowHistoryEntry { heading, body })
        .collect::<Vec<_>>();

    Ok(WorkflowSessionDetail {
        session: summary,
        history,
    })
}

fn read_session_summary(config: &Config, uri: &str) -> Result<WorkflowSessionSummary> {
    let (frontmatter, content, _) = read_markdown(uri, config)?;
    let frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!("Invalid workflow session file format for {uri}"))
    })?;

    let id = uri.rsplit('/').next().unwrap_or_default().to_string();
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let raw_current_step = frontmatter_usize(&frontmatter, "currentStep");
    let total_steps = max_step_total(&content);
    let current_step = if total_steps == 0 {
        raw_current_step
    } else {
        (raw_current_step + 1).min(total_steps)
    };
    let workflow_id = queue.get(current_workflow_index).cloned();
    let status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or("active")
        .to_string();
    let started_at = extract_started_at(&content);
    let updated_at = uri_to_path(uri, &config.memory_path)
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|meta| modified_timestamp(meta.modified().ok()?))
        .unwrap_or_else(current_iso_timestamp);
    let step_timings = read_step_timings(&frontmatter);

    Ok(WorkflowSessionSummary {
        id,
        uri: uri.to_string(),
        status,
        queue,
        workflow_id,
        current_workflow_index,
        current_step,
        total_steps,
        started_at,
        updated_at,
        step_timings,
    })
}

fn extract_started_at(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Started: ")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn max_step_total(content: &str) -> usize {
    extract_h2_sections(content)
        .into_iter()
        .filter_map(|(heading, _)| parse_step_total(&heading))
        .max()
        .unwrap_or(0)
}

fn parse_step_total(heading: &str) -> Option<usize> {
    let remainder = heading.trim().strip_prefix("Step ")?;
    let (_, tail) = remainder.split_once('/')?;
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
    }
}

fn parse_session_id_from_resource_uri(uri: &str) -> Result<String> {
    let encoded = uri
        .strip_prefix(WORKFLOW_SESSION_URI_PREFIX)
        .ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected URI prefix '{WORKFLOW_SESSION_URI_PREFIX}'"
            ))
        })?;
    let decoded = urlencoding::decode(encoded)
        .map_err(|error| {
            MaenifoldError::InvalidResourceUri(format!(
                "Failed to decode workflow session id '{encoded}': {error}"
            ))
        })?
        .to_string();
    let trimmed = decoded.trim();
    if trimmed.is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Missing workflow session id segment"
        )));
    }
    Ok(trimmed.to_string())
}

fn make_resource(uri: &str, name: &str, description: &str) -> Resource {
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    resource.title = Some(name.to_string());
    resource.description = Some(description.to_string());
    resource
}

fn modified_timestamp(value: SystemTime) -> Option<String> {
    let datetime: chrono::DateTime<chrono::Utc> = value.into();
    Some(datetime.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true))
}

fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}
