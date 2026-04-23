use crate::api::routes::{ApiError, ApiResult, ApiState};
use agent_client_protocol as acp;
use axum::{
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

use super::acp_relay_dispatch::{apply_event_state, forward_event, handle_incoming};
use super::acp_relay_helpers::{cancel_permissions, rpc_error, rpc_ok, send_json};
use super::acp_relay_mcp::load_configured_mcp_servers;

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
}

#[derive(Clone)]
pub(super) struct RelayConfig {
    pub(super) agent_name: String,
    pub(super) adapter: jockey::acp::StdioAdapterSpec,
    pub(super) default_cwd: String,
    /// MCP servers from maenifold.json (global + per-agent), pre-converted to ACP schema.
    pub(super) configured_mcp_servers: Vec<acp::McpServer>,
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
        .map_err(ApiError::bad_request)?
        .ok_or_else(|| ApiError::bad_request(format!("agent '{agent_name}' is not configured")))?;
    let root = jockey::acp::config_root();
    let default_cwd = jockey::acp::resolve_cwd(root.to_string_lossy().as_ref());
    let configured_mcp_servers = load_configured_mcp_servers(&root, &agent_name);
    Ok(ws.on_upgrade(move |socket| {
        relay_socket(
            socket,
            RelayConfig {
                agent_name,
                adapter,
                default_cwd,
                configured_mcp_servers,
            },
        )
    }))
}

async fn relay_socket(socket: WebSocket, relay: RelayConfig) {
    let (mut sender, mut receiver) = socket.split();
    let mut session: Option<RelaySession> = None;
    let mut pending_prompt: Option<PendingPrompt> = None;
    let mut delta_rx: Option<mpsc::Receiver<jockey::acp::AcpEvent>> = None;
    let mut result_rx: Option<oneshot::Receiver<Result<(String, String), String>>> = None;
    // NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
    // The code is retained for future multi-user support but is currently unreachable.
    let mut permission_ids: HashMap<u64, String> = HashMap::new();
    let mut next_permission_id = 1_u64;

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else { break; };
                if !handle_incoming(
                    message,
                    &relay,
                    &mut sender,
                    &mut session,
                    &mut pending_prompt,
                    &mut delta_rx,
                    &mut result_rx,
                    &mut permission_ids,
                    &mut next_permission_id,
                ).await {
                    break;
                }
            }
            event = async { delta_rx.as_mut().unwrap().recv().await }, if delta_rx.is_some() => {
                let Some(event) = event else { delta_rx = None; continue; };
                if let Some(active) = session.as_mut() {
                    apply_event_state(active, &event);
                    if !forward_event(
                        &mut sender,
                        &active.session_id,
                        &event,
                        &mut permission_ids,
                        &mut next_permission_id,
                    ).await {
                        break;
                    }
                }
            }
            result = async { result_rx.as_mut().unwrap().await }, if result_rx.is_some() => {
                let result = result.unwrap_or_else(|_| Err("worker disconnected".to_string()));
                if let Some(session_id) = session.as_ref().map(|active| active.session_id.clone()) {
                    while let Some(event) = delta_rx.as_mut().and_then(|rx| rx.try_recv().ok()) {
                        if let Some(active_mut) = session.as_mut() {
                            apply_event_state(active_mut, &event);
                        }
                        if !forward_event(
                            &mut sender,
                            &session_id,
                            &event,
                            &mut permission_ids,
                            &mut next_permission_id,
                        ).await {
                            break;
                        }
                    }
                }
                let cancelled = pending_prompt.as_ref().is_some_and(|prompt| prompt.cancelled);
                let request_id = pending_prompt.take().map(|prompt| prompt.request_id);
                delta_rx = None;
                result_rx = None;
                if let Some(id) = request_id {
                    let payload = match result {
                        Ok(_) => rpc_ok(
                            id,
                            json!({"stopReason": if cancelled { "cancelled" } else { "end_turn" }}),
                        ),
                        Err(error) => rpc_error(id, -32000, error),
                    };
                    if !send_json(&mut sender, payload).await {
                        break;
                    }
                }
            }
        }
    }

    if let Some(active) = session.as_ref() {
        let _ = jockey::acp::worker_tx().send(jockey::acp::WorkerMsg::Cancel {
            runtime_key: relay.adapter.runtime_key.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: active.app_session_id.clone(),
        });
    }
    cancel_permissions(permission_ids);
}
