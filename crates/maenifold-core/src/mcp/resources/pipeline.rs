use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{
    AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use serde_json::{json, Value};

const PIPELINE_SESSIONS_URI: &str = "pipeline://sessions";
const LEGACY_PIPELINE_SESSIONS_URI: &str = "pipeline://sessions/active";
const PIPELINE_SESSION_PREFIX: &str = "pipeline://session/";
const PIPELINE_NODE_PREFIX: &str = "pipeline://node/";

pub(super) fn list_resources() -> Result<Vec<Resource>> {
    let mut resources = vec![RawResource::new(PIPELINE_SESSIONS_URI, "Pipeline sessions")
        .with_title("Pipeline sessions")
        .with_description("List of pipeline sessions for the dashboard.")
        .with_mime_type("application/json")
        .optional_annotate(None)];

    let sessions_payload = read_sessions_payload()?;
    if let Some(sessions) = sessions_payload.get("sessions").and_then(Value::as_array) {
        for session in sessions {
            let Some(session_id) = session
                .get("pipelineId")
                .or_else(|| session.get("id"))
                .and_then(Value::as_str)
            else {
                continue;
            };

            resources.push(
                RawResource::new(
                    format!("{PIPELINE_SESSION_PREFIX}{session_id}"),
                    format!("Pipeline session {session_id}"),
                )
                .with_title(session_id.to_string())
                .with_description("Current state for one pipeline session.")
                .with_mime_type("application/json")
                .optional_annotate(None),
            );
        }
    }

    Ok(resources)
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        RawResourceTemplate::new("pipeline://session/{id}", "Pipeline session")
            .with_title("Pipeline session")
            .with_description("Template for one pipeline session state.")
            .with_mime_type("application/json")
            .optional_annotate(None),
        RawResourceTemplate::new(
            "pipeline://node/{session_id}/{node_id}",
            "Pipeline session node",
        )
        .with_title("Pipeline node")
        .with_description("Template for one node within a pipeline session.")
        .with_mime_type("application/json")
        .optional_annotate(None),
    ]
}

pub(super) fn is_pipeline_uri(uri: &str) -> bool {
    uri.starts_with("pipeline://")
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if uri == PIPELINE_SESSIONS_URI || uri == LEGACY_PIPELINE_SESSIONS_URI {
        return read_json_resource(uri, read_sessions_payload()?);
    }

    if let Some(session_id) = uri.strip_prefix(PIPELINE_SESSION_PREFIX) {
        validate_uri_component("session id", session_id)?;
        return read_json_resource(uri, read_session_payload(session_id)?);
    }

    if let Some(rest) = uri.strip_prefix(PIPELINE_NODE_PREFIX) {
        let Some((session_id, node_id)) = rest.split_once('/') else {
            return Err(MaenifoldError::InvalidResourceUri(uri.to_string()));
        };
        validate_uri_component("session id", session_id)?;
        validate_uri_component("node id", node_id)?;
        return read_json_resource(uri, read_node_payload(session_id, node_id)?);
    }

    Err(MaenifoldError::InvalidResourceUri(format!(
        "{uri}. Expected '{PIPELINE_SESSIONS_URI}', '{PIPELINE_SESSION_PREFIX}{{id}}', or '{PIPELINE_NODE_PREFIX}{{session_id}}/{{node_id}}'"
    )))
}

fn validate_uri_component(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('/') {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid {label} in pipeline resource URI"
        )));
    }
    Ok(())
}

fn read_node_payload(session_id: &str, node_id: &str) -> Result<Value> {
    let session_payload = read_session_payload(session_id)?;
    let nodes = session_payload
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Pipeline session {session_id} is missing node data for resource lookup"
            ))
        })?;

    let node = nodes
        .iter()
        .find(|entry| {
            entry
                .get("nodeId")
                .or_else(|| entry.get("id"))
                .and_then(Value::as_str)
                == Some(node_id)
        })
        .cloned()
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Node {node_id} not found in pipeline session {session_id}"
            ))
        })?;

    let mut response = json!({
        "sessionId": session_id,
        "nodeId": node_id,
    });
    if let (Some(response_object), Some(node_object)) = (response.as_object_mut(), node.as_object())
    {
        response_object.extend(node_object.clone());
    } else {
        response["node"] = node;
    }
    Ok(response)
}

fn read_session_payload(session_id: &str) -> Result<Value> {
    read_pipeline_tool_data(json!({
        "sessionId": session_id,
        "view": true,
    }))
}

fn read_sessions_payload() -> Result<Value> {
    let payload = read_pipeline_tool_data(json!({ "list": true }))?;
    if payload.get("sessions").is_some() {
        return Ok(payload);
    }

    let sessions = payload
        .get("pipelines")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({ "sessions": sessions }))
}

fn read_pipeline_tool_data(payload: Value) -> Result<Value> {
    let output = ToolRegistry::global().try_invoke("Pipeline", &payload, OutputFormat::Json)?;
    let parsed: Value = serde_json::from_str(&output).map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to decode pipeline tool response for resource payload: {error}"
        ))
    })?;

    if parsed
        .get("success")
        .and_then(Value::as_bool)
        .is_some_and(|success| !success)
    {
        let message = parsed
            .get("error")
            .or_else(|| parsed.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Pipeline resource request failed");
        return Err(MaenifoldError::Message(message.to_string()));
    }

    Ok(parsed.get("data").cloned().unwrap_or(parsed))
}

fn read_json_resource(uri: &str, payload: Value) -> Result<ReadResourceResult> {
    let text = serde_json::to_string_pretty(&payload)?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        text,
        uri.to_string(),
    )
    .with_mime_type("application/json")]))
}
