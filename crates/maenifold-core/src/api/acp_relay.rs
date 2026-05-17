use crate::api::routes::{ApiError, ApiResult, ApiState};
use agent_client_protocol as acp;
use axum::{
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::oneshot;

#[cfg(test)]
use super::acp_relay_lifecycle::{
    prompt_result_payload, relay_idle_cleanup_ttl, relay_reset_message,
};
use super::acp_relay_lifecycle::{relay_reset_by_session_message, relay_socket};
use super::acp_relay_mcp::load_configured_mcp_servers;

// Keep-alive relay sessions are retained briefly so a renderer can reconnect
// after a transient WebSocket drop. No daemon setting exists for this relay yet;
// keep this local default until ACP relay settings are added.
pub(super) const RELAY_IDLE_CLEANUP_TTL: Duration = Duration::from_secs(5 * 60);

// ── Structured config option types (ACP protocol § SessionConfigOption) ──────
//
// The relay stores config options as `Vec<Value>` to faithfully preserve the
// full ACP `SessionConfigOption` structure (including grouped select options
// and `_meta`).  These types provide typed access for category-aware operations
// without requiring full deserialization.
//
// For typed access to select options and groups, use the ACP schema types
// directly: `acp::SessionConfigSelectOption`, `acp::SessionConfigSelectGroup`.

/// Semantic category for a session configuration option.
///
/// Mirrors `SessionConfigOptionCategory` from the ACP protocol.  Clients use
/// this for UX decisions — e.g. which icon to show, where to place the
/// selector — without string-matching on config IDs.
///
/// Custom categories prefixed with `_` are free for use (e.g. `_pipeline_mode`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ConfigCategory {
    /// Model selector (e.g. GPT-4o, Claude Sonnet).
    Model,
    /// Session mode selector (e.g. agent, edit, ask).
    Mode,
    /// Thought/reasoning level selector (e.g. low, medium, high).
    ThoughtLevel,
    /// Unknown or custom category (including `_`-prefixed extensions).
    #[serde(untagged)]
    Other(String),
}

/// Extract the semantic [`ConfigCategory`] from a config option JSON value.
///
/// Returns `None` when the option has no `category` field or the value cannot
/// be parsed (which is fine — the protocol says category is optional and
/// clients MUST handle missing/unknown categories gracefully).
pub(super) fn config_option_category(option: &Value) -> Option<ConfigCategory> {
    option
        .get("category")
        .and_then(|v| serde_json::from_value::<ConfigCategory>(v.clone()).ok())
}

#[derive(Deserialize)]
pub struct RelayQuery {
    agent: Option<String>,
    #[serde(default, rename = "keepSessionAlive")]
    keep_session_alive: bool,
}

#[derive(Clone)]
pub(super) struct RelayConfig {
    pub(super) agent_name: String,
    pub(super) adapter: jockey::acp::StdioAdapterSpec,
    pub(super) default_cwd: String,
    /// MCP servers from maenifold.json (global + per-agent), pre-converted to ACP schema.
    pub(super) configured_mcp_servers: Vec<acp::McpServer>,
    pub(super) keep_session_alive: bool,
}

pub(super) struct RelaySession {
    pub(super) app_session_id: String,
    pub(super) session_id: String,
    pub(super) cwd: String,
    pub(super) mcp_servers: Vec<acp::McpServer>,
    pub(super) client_capabilities: acp::ClientCapabilities,
    pub(super) config_options: Vec<Value>,
    pub(super) modes: Vec<String>,
    pub(super) current_mode_id: Option<String>,
}

pub(super) struct PendingPrompt {
    pub(super) request_id: Value,
    pub(super) cancelled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SessionSetupParams {
    pub(super) cwd: Option<PathBuf>,
    #[serde(default)]
    pub(super) mcp_servers: Vec<acp::McpServer>,
    #[serde(default)]
    pub(super) config_options: HashMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SessionLoadParams {
    pub(super) session_id: String,
    pub(super) cwd: Option<PathBuf>,
    #[serde(default)]
    pub(super) mcp_servers: Vec<acp::McpServer>,
}

fn default_agent_cwd(root: &std::path::Path) -> ApiResult<String> {
    let workdir = root.join("acp-workdir");
    std::fs::create_dir_all(&workdir).map_err(|error| {
        ApiError::internal(format!(
            "Failed to create ACP agent workdir '{}': {error}",
            workdir.display()
        ))
    })?;
    Ok(jockey::acp::resolve_cwd(workdir.to_string_lossy().as_ref()))
}

/// Handles ACP relay WebSocket upgrades. The `?agent=` parameter must match a
/// key in `~/maenifold/maenifold.json`'s `agents` block using a
/// case-insensitive comparison.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<ApiState>,
    Query(query): Query<RelayQuery>,
) -> ApiResult<impl IntoResponse> {
    let agent_name = query
        .agent
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("Missing ?agent="))?;
    let adapter = jockey::acp::build_stdio_adapter(&agent_name)
        .map_err(|e| {
            ApiError::bad_request(format!(
                "{e} — add it under the \"agents\" key to enable ACP relay"
            ))
        })?
        .ok_or_else(|| ApiError::bad_request(format!("agent '{agent_name}' is not configured")))?;
    let root = jockey::acp::config_root();
    let default_cwd = default_agent_cwd(&root)?;
    let configured_mcp_servers = load_configured_mcp_servers(&root, &agent_name);
    Ok(ws.on_upgrade(move |socket| {
        relay_socket(
            socket,
            RelayConfig {
                agent_name,
                adapter,
                default_cwd,
                configured_mcp_servers,
                keep_session_alive: query.keep_session_alive,
            },
        )
    }))
}

pub async fn close_session_handler(
    Path(session_id): Path<String>,
    Query(query): Query<RelayQuery>,
) -> ApiResult<impl IntoResponse> {
    let agent_name = query
        .agent
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("Missing ?agent="))?;
    let adapter = jockey::acp::build_stdio_adapter(&agent_name)
        .map_err(|e| {
            ApiError::bad_request(format!(
                "{e} — add it under the \"agents\" key to enable ACP relay"
            ))
        })?
        .ok_or_else(|| ApiError::bad_request(format!("agent '{agent_name}' is not configured")))?;
    let (result_tx, result_rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(relay_reset_by_session_message(
            adapter.runtime_key,
            agent_name,
            session_id,
            result_tx,
        ))
        .map_err(|_| ApiError::internal("ACP worker unavailable"))?;
    let closed = result_rx
        .await
        .map_err(|_| ApiError::internal("ACP worker disconnected"))?
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "closed": closed })))
}

#[cfg(test)]
#[path = "acp_relay_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "acp_relay_split_tests.rs"]
mod split_tests;
