use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{
    AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const AGENT_SESSIONS_URI: &str = "agent://sessions";
pub const AGENT_SESSION_TEMPLATE_URI: &str = "agent://session/{id}";
const AGENT_SESSION_URI_PREFIX: &str = "agent://session/";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AgentSessionSummary {
    session_id: String,
    agent_name: String,
    cwd: String,
    title: Option<String>,
    created_at: Option<String>,
    last_activity: Option<String>,
    updated_at: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionListPayload {
    sessions: Vec<AgentSessionSummary>,
    errors: Vec<Value>,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionDetailPayload {
    session: AgentSessionSummary,
    generated_at: String,
}

pub(super) fn list_resources() -> Vec<Resource> {
    vec![make_resource(
        AGENT_SESSIONS_URI,
        "Agent Sessions",
        "ACP agent sessions discovered across configured agents.",
    )]
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![make_template(
        AGENT_SESSION_TEMPLATE_URI,
        "Agent Session Detail",
        "Session detail for a single ACP session by id.",
    )]
}

pub(super) fn is_agent_uri(uri: &str) -> bool {
    uri.starts_with("agent://")
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let sessions = list_sessions()?;
    let generated_at = current_iso_timestamp();

    let body = if uri == AGENT_SESSIONS_URI {
        serde_json::to_string_pretty(&AgentSessionListPayload {
            sessions,
            errors: list_session_errors()?,
            generated_at,
        })?
    } else if let Some(session_id) = uri.strip_prefix(AGENT_SESSION_URI_PREFIX) {
        if session_id.trim().is_empty() {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected 'agent://session/{{id}}' with a non-empty id"
            )));
        }

        let session = sessions
            .into_iter()
            .find(|item| item.session_id == session_id)
            .ok_or_else(|| {
                MaenifoldError::AssetNotFound("agent-session".to_string(), session_id.to_string())
            })?;

        serde_json::to_string_pretty(&AgentSessionDetailPayload {
            session,
            generated_at,
        })?
    } else {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{AGENT_SESSIONS_URI}' or '{AGENT_SESSION_TEMPLATE_URI}'"
        )));
    };

    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        body,
        uri.to_string(),
    )
    .with_mime_type("application/json")]))
}

fn list_sessions() -> Result<Vec<AgentSessionSummary>> {
    let data = list_sessions_data()?;
    let sessions_value = data.get("sessions").ok_or_else(|| {
        MaenifoldError::InvalidPayload("ListSessions response missing data.sessions".to_string())
    })?;
    let sessions = serde_json::from_value::<Vec<AgentSessionSummary>>(sessions_value.clone())?;
    Ok(sessions)
}

fn list_session_errors() -> Result<Vec<Value>> {
    let data = list_sessions_data()?;
    match data.get("errors") {
        Some(Value::Array(items)) => Ok(items.clone()),
        Some(_) => Err(MaenifoldError::InvalidPayload(
            "ListSessions response data.errors must be an array".to_string(),
        )),
        None => Ok(Vec::new()),
    }
}

fn list_sessions_data() -> Result<Value> {
    let raw = ToolRegistry::global().try_invoke("ListSessions", &json!({}), OutputFormat::Json)?;
    let parsed = serde_json::from_str::<Value>(&raw)?;

    if parsed
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return parsed.get("data").cloned().ok_or_else(|| {
            MaenifoldError::InvalidPayload("ListSessions success payload missing data".to_string())
        });
    }

    if parsed.get("sessions").is_some() {
        return Ok(parsed);
    }

    if let Some(error_message) = parsed
        .get("error")
        .and_then(Value::as_object)
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
    {
        return Err(MaenifoldError::Message(error_message.to_string()));
    }

    Err(MaenifoldError::InvalidPayload(
        "Unexpected ListSessions payload shape".to_string(),
    ))
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

fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}
