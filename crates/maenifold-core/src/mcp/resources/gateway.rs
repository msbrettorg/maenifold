use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::{ServerStartupStatus, StdioPool};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use serde_json::json;
use std::sync::Arc;

const GATEWAY_DIAGNOSTICS_URI: &str = "gateway://diagnostics";

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new(GATEWAY_DIAGNOSTICS_URI, "Gateway startup diagnostics")
            .with_title("Gateway startup diagnostics")
            .with_description("Per-server gateway startup health and sanitized stderr tails.")
            .with_mime_type("application/json")
            .optional_annotate(None),
    ]
}

pub(super) fn is_gateway_uri(uri: &str) -> bool {
    uri == GATEWAY_DIAGNOSTICS_URI
}

pub(super) async fn handle_read_resource(
    uri: &str,
    diagnostics: Option<Arc<StdioPool>>,
) -> Result<ReadResourceResult> {
    if !is_gateway_uri(uri) {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{GATEWAY_DIAGNOSTICS_URI}'"
        )));
    }

    let pool = diagnostics.ok_or_else(|| {
        MaenifoldError::InvalidArgument("gateway diagnostics are not available".to_string())
    })?;
    let health = pool.health().await;
    let servers = health
        .servers
        .into_iter()
        .map(|server| {
            json!({
                "name": server.name,
                "status": match server.status {
                    ServerStartupStatus::Started => "started",
                    ServerStartupStatus::Failed => "failed",
                },
                "toolCount": server.tool_count,
                "error": server.last_start_error,
                "stderrTail": server.stderr_tail,
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({ "servers": servers });
    let text = serde_json::to_string_pretty(&payload)?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri.to_string()).with_mime_type("application/json"),
    ]))
}
