use std::sync::{Arc, RwLock};

use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use serde::Serialize;

use crate::error::{MaenifoldError, Result};
use crate::mcp::SamplingSessionRegistry;

pub const SAMPLING_SESSIONS_URI: &str = "sampling://sessions";

static SAMPLING_REGISTRY: RwLock<Option<Arc<SamplingSessionRegistry>>> = RwLock::new(None);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SamplingSessionsPayload {
    sessions: Vec<crate::mcp::sampling_session_registry::SamplingDispatch>,
    generated_at: String,
}

pub(crate) fn set_registry(registry: Arc<SamplingSessionRegistry>) {
    if let Ok(mut guard) = SAMPLING_REGISTRY.write() {
        *guard = Some(registry);
    }
}

pub(crate) fn registry() -> Option<Arc<SamplingSessionRegistry>> {
    SAMPLING_REGISTRY
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

pub fn list_resources() -> Vec<Resource> {
    vec![make_resource(
        SAMPLING_SESSIONS_URI,
        "Sampling Sessions",
        "Runtime MCP sampling dispatch sessions and cancellation state.",
    )]
}

pub fn is_sampling_uri(uri: &str) -> bool {
    uri == SAMPLING_SESSIONS_URI
}

pub fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if !is_sampling_uri(uri) {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{SAMPLING_SESSIONS_URI}'"
        )));
    }
    let Some(registry) = registry() else {
        return Err(MaenifoldError::Message(
            "sampling session registry is not initialized".to_string(),
        ));
    };

    let sessions = read_snapshot(registry)?;
    let body = serde_json::to_string_pretty(&SamplingSessionsPayload {
        sessions,
        generated_at: current_iso_timestamp(),
    })?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(body, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn read_snapshot(
    registry: Arc<SamplingSessionRegistry>,
) -> Result<Vec<crate::mcp::sampling_session_registry::SamplingDispatch>> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Ok(tokio::task::block_in_place(|| {
            handle.block_on(registry.snapshot())
        })),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| MaenifoldError::Message(error.to_string()))?;
            Ok(runtime.block_on(registry.snapshot()))
        }
    }
}

fn make_resource(uri: &str, name: &str, description: &str) -> Resource {
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    resource.title = Some(name.to_string());
    resource.description = Some(description.to_string());
    resource
}

fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}
