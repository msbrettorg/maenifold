use crate::acp_config::AgentsConfig;
use crate::api::routes::{ApiError, ApiResult, ApiState};
use agent_client_protocol as acp;
use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

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
pub(crate) enum ConfigCategory {
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
pub(crate) fn config_option_category(option: &Value) -> Option<ConfigCategory> {
    option
        .get("category")
        .and_then(|v| serde_json::from_value::<ConfigCategory>(v.clone()).ok())
}

/// Extract the config option type discriminator (`"select"`, `"boolean"`, etc.)
/// from a config option JSON value.
fn config_option_type(option: &Value) -> Option<&str> {
    option.get("type").and_then(Value::as_str)
}

#[derive(Deserialize)]
pub struct RelayQuery {
    agent: Option<String>,
}

#[derive(Clone)]
struct RelayConfig {
    agent_name: String,
    adapter: jockey::acp::StdioAdapterSpec,
    default_cwd: String,
    /// MCP servers from maenifold.json (global + per-agent), pre-converted to ACP schema.
    configured_mcp_servers: Vec<acp::McpServer>,
}

struct RelaySession {
    app_session_id: String,
    session_id: String,
    cwd: String,
    mcp_servers: Vec<acp::McpServer>,
    client_capabilities: acp::ClientCapabilities,
    config_options: Vec<Value>,
    modes: Vec<String>,
    current_mode_id: Option<String>,
}

struct PendingPrompt {
    request_id: Value,
    cancelled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionSetupParams {
    cwd: Option<PathBuf>,
    #[serde(default)]
    mcp_servers: Vec<acp::McpServer>,
    #[serde(default)]
    config_options: HashMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionLoadParams {
    session_id: String,
    cwd: Option<PathBuf>,
    #[serde(default)]
    mcp_servers: Vec<acp::McpServer>,
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

#[allow(clippy::too_many_arguments)]
async fn handle_incoming(
    message: Message,
    relay: &RelayConfig,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    session: &mut Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    delta_rx: &mut Option<mpsc::Receiver<jockey::acp::AcpEvent>>,
    result_rx: &mut Option<oneshot::Receiver<Result<(String, String), String>>>,
    permission_ids: &mut HashMap<u64, String>,
    next_permission_id: &mut u64,
) -> bool {
    let text = match message {
        Message::Text(text) => text,
        Message::Close(_) => return false,
        _ => return true,
    };
    let Ok(payload) = serde_json::from_str::<Value>(&text) else {
        return true;
    };
    let Some(object) = payload.as_object() else {
        return true;
    };

    // NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
    // The code is retained for future multi-user support but is currently unreachable.
    if let Some(id) = object.get("id").and_then(Value::as_u64) {
        if object.get("method").is_none() {
            if let Some(request_id) = permission_ids.remove(&id) {
                let outcome = object
                    .get("result")
                    .cloned()
                    .and_then(|value| {
                        serde_json::from_value::<acp::RequestPermissionResponse>(value).ok()
                    })
                    .map(|response| response.outcome)
                    .unwrap_or(acp::RequestPermissionOutcome::Cancelled);
                jockey::acp::respond_to_permission(&request_id, outcome);
                return true;
            }
        }
    }

    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return true;
    };
    let id = object.get("id").cloned();
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

    let response = match method {
        "initialize" if id.is_some() => Some(
            handle_initialize(relay, session, params, id.clone().unwrap())
                .await
                .map(|result| rpc_ok(id.unwrap(), result)),
        ),
        "session/new" if id.is_some() => Some(
            handle_session_new(relay, session, params, id.clone().unwrap())
                .await
                .map(|result| rpc_ok(id.unwrap(), result)),
        ),
        "session/load" if id.is_some() => Some(
            handle_session_load(
                relay,
                sender,
                session,
                permission_ids,
                next_permission_id,
                params,
                id.clone().unwrap(),
            )
            .await
            .map(|result| rpc_ok(id.unwrap(), result)),
        ),
        "session/prompt" if id.is_some() => match handle_prompt(
            relay,
            session,
            pending_prompt,
            delta_rx,
            result_rx,
            params,
            id.unwrap(),
        ) {
            Ok(()) => None,
            Err(error) => Some(Err(error)),
        },
        "session/set_config_option" if id.is_some() => Some(
            handle_set_config_option(relay, session, params, id.clone().unwrap())
                .await
                .map(|result| rpc_ok(id.unwrap(), result)),
        ),
        "session/cancel" => {
            handle_cancel(relay, session, pending_prompt, permission_ids, params);
            None
        }
        _ if id.is_some() => Some(Err(rpc_error(
            id.unwrap(),
            -32601,
            format!("Method not found: {method}"),
        ))),
        _ => None,
    };

    if let Some(response) = response {
        let payload = match response {
            Ok(value) | Err(value) => value,
        };
        return send_json(sender, payload).await;
    }
    true
}

async fn handle_initialize(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let request: acp::InitializeRequest = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    if session.is_none() {
        let app_session_id = format!("acp-relay:{}", uuid::Uuid::new_v4());
        let client_capabilities = request.client_capabilities.clone();
        let mcp_servers = relay.configured_mcp_servers.clone();
        let (config_options, modes, session_id) = prewarm_session(
            relay,
            &app_session_id,
            relay.default_cwd.clone(),
            mcp_servers.clone(),
            client_capabilities.clone(),
            None,
            HashMap::new(),
            request_id.clone(),
        )
        .await?;
        if session_id.is_empty() {
            return Err(rpc_error(
                request_id,
                -32603,
                "Agent initialization failed — check agent configuration and binary path",
            ));
        }
        *session = Some(RelaySession {
            app_session_id,
            session_id,
            cwd: relay.default_cwd.clone(),
            mcp_servers,
            client_capabilities,
            config_options,
            modes,
            current_mode_id: None,
        });
    } else if let Some(active) = session.as_mut() {
        active.client_capabilities = request.client_capabilities;
    }

    Ok(json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": true,
            "promptCapabilities": { "image": false, "audio": false, "embeddedContext": true },
            "sessionCapabilities": {},
            "_meta": { "maenifold/extensions": { "supportedAgentMethods": ["session/set_config_option"] } }
        },
        "authMethods": [],
        "agentInfo": { "name": relay.agent_name, "title": relay.agent_name, "version": "0.1.0" }
    }))
}

async fn handle_session_new(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/new",
        ));
    };
    let app_session_id = active.app_session_id.clone();
    let client_capabilities = active.client_capabilities.clone();
    let request: SessionSetupParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    let cwd = resolve_session_cwd(request.cwd, &relay.default_cwd)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    reset_session_connection(relay, &app_session_id, request_id.clone()).await?;
    let mcp_servers = merge_mcp_servers(&relay.configured_mcp_servers, request.mcp_servers);
    let session_config_options = request.config_options;
    let (config_options, modes, session_id) = prewarm_session(
        relay,
        &app_session_id,
        cwd.clone(),
        mcp_servers.clone(),
        client_capabilities,
        None,
        session_config_options,
        request_id.clone(),
    )
    .await?;
    if session_id.is_empty() {
        return Err(rpc_error(
            request_id,
            -32603,
            "session/new failed — agent did not return a session",
        ));
    }
    let active = session
        .as_mut()
        .ok_or_else(|| rpc_error(request_id.clone(), -32600, "relay session disappeared"))?;
    active.session_id = session_id.clone();
    active.cwd = cwd;
    active.mcp_servers = mcp_servers;
    active.config_options = config_options.clone();
    active.modes = modes.clone();
    active.current_mode_id = None;
    let modes = build_modes(&active.modes, &active.current_mode_id);
    Ok(json!({
        "sessionId": session_id,
        "configOptions": config_options,
        "availableCommands": jockey::acp::list_available_commands(
            &app_session_id,
            &relay.adapter.runtime_key,
            &relay.agent_name,
        ),
        "modes": modes,
    }))
}

async fn handle_session_load(
    relay: &RelayConfig,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    session: &mut Option<RelaySession>,
    permission_ids: &mut HashMap<u64, String>,
    next_permission_id: &mut u64,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let active = session.as_ref().ok_or_else(|| {
        rpc_error(
            request_id.clone(),
            -32600,
            "initialize must be called before session/load",
        )
    })?;
    let request: SessionLoadParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    let requested_session_id = request.session_id;
    let app_session_id = active.app_session_id.clone();
    let client_capabilities = active.client_capabilities.clone();
    let previous_cwd = active.cwd.clone();
    let cwd = request
        .cwd
        .map(|cwd| {
            resolve_session_cwd(Some(cwd), &relay.default_cwd)
                .map_err(|error| rpc_error(request_id.clone(), -32602, error))
        })
        .transpose()?
        .unwrap_or(previous_cwd);
    let mcp_servers = merge_mcp_servers(&relay.configured_mcp_servers, request.mcp_servers);

    // Reset the current connection so the pool key is free for re-prewarm
    reset_session_connection(relay, &app_session_id, request_id.clone()).await?;

    let (event_tx, mut event_rx) = mpsc::channel(512);
    let (tx, mut rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Prewarm {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            client_capabilities: client_capabilities.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.clone(),
            cwd: cwd.clone(),
            auto_approve: true,
            mcp_servers: mcp_servers.clone(),
            role_mode: None,
            role_config_options: vec![],
            delta_tx: Some(event_tx),
            result_tx: Some(tx),
            resume_session_id: Some(requested_session_id.clone()),
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;

    let mut replay_complete = false;
    let mut replay_result = None;
    let (config_options, modes, session_id) = loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
                    return Err(rpc_error(request_id.clone(), -32000, "ACP replay channel closed before completion"));
                };
                if matches!(event, jockey::acp::AcpEvent::ReplayComplete) {
                    replay_complete = true;
                    if let Some(result) = replay_result.take() {
                        break result;
                    }
                    continue;
                }
                if !replay_complete
                    && !forward_event(
                        sender,
                        &requested_session_id,
                        &event,
                        permission_ids,
                        next_permission_id,
                    ).await
                {
                    return Err(rpc_error(request_id.clone(), -32000, "websocket closed during session/load replay"));
                }
            }
            result = &mut rx => {
                let result = result
                    .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?;
                if replay_complete {
                    break result;
                }
                replay_result = Some(result);
            }
        }
    };
    if session_id.is_empty() {
        return Err(rpc_error(
            request_id,
            -32603,
            "session/load failed — agent did not return a session",
        ));
    }
    *session = Some(RelaySession {
        app_session_id: app_session_id.clone(),
        session_id: session_id.clone(),
        cwd,
        mcp_servers,
        client_capabilities,
        config_options: config_options.clone(),
        modes: modes.clone(),
        current_mode_id: None,
    });
    let modes = session
        .as_ref()
        .and_then(|active| build_modes(&active.modes, &active.current_mode_id));
    Ok(json!({
        "sessionId": session_id,
        "configOptions": config_options,
        "availableCommands": jockey::acp::list_available_commands(
            &app_session_id,
            &relay.adapter.runtime_key,
            &relay.agent_name,
        ),
        "modes": modes,
    }))
}

fn handle_prompt(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    delta_rx: &mut Option<mpsc::Receiver<jockey::acp::AcpEvent>>,
    result_rx: &mut Option<oneshot::Receiver<Result<(String, String), String>>>,
    params: Value,
    request_id: Value,
) -> Result<(), Value> {
    if pending_prompt.is_some() {
        return Err(rpc_error(
            request_id,
            -32000,
            "session/prompt already in progress",
        ));
    }
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/prompt",
        ));
    };
    let request: acp::PromptRequest = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    if request.session_id.to_string() != active.session_id {
        return Err(rpc_error(
            request_id,
            -32602,
            "sessionId does not match active session",
        ));
    }
    let (prompt, context) =
        prompt_parts(&request).map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    let (event_tx, event_rx) = mpsc::channel(512);
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Execute {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: active.app_session_id.clone(),
            prompt,
            context,
            cwd: active.cwd.clone(),
            delta_tx: event_tx,
            result_tx: tx,
            // Design decision: auto_approve is always true for the single-user desktop app.
            // The user is the operator — click-through permissions on every fs/write would
            // make pipelines unusable. When multi-user support is needed, this should be
            // configurable via relay settings.
            auto_approve: true,
            mcp_servers: active.mcp_servers.clone(),
            client_capabilities: active.client_capabilities.clone(),
            role_mode: None,
            role_config_options: vec![],
            resume_session_id: None,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    *pending_prompt = Some(PendingPrompt {
        request_id: request_id.clone(),
        cancelled: false,
    });
    *delta_rx = Some(event_rx);
    *result_rx = Some(rx);
    Ok(())
}

async fn handle_set_config_option(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let Some(active) = session.as_mut() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/set_config_option",
        ));
    };
    let request = parse_set_config_request(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    if request.0 != active.session_id {
        return Err(rpc_error(
            request_id.clone(),
            -32602,
            "sessionId does not match active session",
        ));
    }
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::SetConfigOption {
            runtime_key: relay.adapter.runtime_key.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: active.app_session_id.clone(),
            config_id: request.1.clone(),
            value: request.2.clone(),
            result_tx: tx,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    rx.await
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?
        .map_err(|error| rpc_error(request_id.clone(), -32000, error))?;
    let category = update_config_options(&mut active.config_options, &request.1, &request.2);
    Ok(json!({
        "configOptions": active.config_options,
        "updatedCategory": category,
    }))
}

fn handle_cancel(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    permission_ids: &mut HashMap<u64, String>,
    params: Value,
) {
    let Some(active) = session.as_ref() else {
        return;
    };
    let Ok(request) = serde_json::from_value::<acp::CancelNotification>(params) else {
        return;
    };
    if request.session_id.to_string() != active.session_id {
        return;
    }
    if let Some(prompt) = pending_prompt.as_mut() {
        prompt.cancelled = true;
    }
    cancel_permissions(std::mem::take(permission_ids));
    let _ = jockey::acp::worker_tx().send(jockey::acp::WorkerMsg::Cancel {
        runtime_key: relay.adapter.runtime_key.clone(),
        role_name: relay.agent_name.clone(),
        app_session_id: active.app_session_id.clone(),
    });
}

fn apply_event_state(session: &mut RelaySession, event: &jockey::acp::AcpEvent) {
    match event {
        jockey::acp::AcpEvent::ConfigUpdate { options } => session.config_options = options.clone(),
        jockey::acp::AcpEvent::AvailableModes { modes, current } => {
            session.modes = modes
                .iter()
                .filter_map(|mode| {
                    mode.get("id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect();
            session.current_mode_id = current.clone();
        }
        jockey::acp::AcpEvent::ModeUpdate { mode_id } => {
            session.current_mode_id = Some(mode_id.clone());
        }
        _ => {}
    }
}

async fn forward_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    session_id: &str,
    event: &jockey::acp::AcpEvent,
    permission_ids: &mut HashMap<u64, String>,
    next_permission_id: &mut u64,
) -> bool {
    let payload = match event {
        jockey::acp::AcpEvent::TextDelta { text } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}}))),
        jockey::acp::AcpEvent::UserTextDelta { text } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "user_message_chunk", "content": {"type": "text", "text": text}}}))),
        jockey::acp::AcpEvent::ThoughtDelta { text } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": text}}}))),
        jockey::acp::AcpEvent::ToolCall { tool_call_id, title, tool_kind, status, content, locations, raw_input, raw_output } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "tool_call", "toolCallId": tool_call_id, "title": title, "kind": tool_kind, "status": status, "content": content, "locations": locations, "rawInput": raw_input, "rawOutput": raw_output}}))),
        jockey::acp::AcpEvent::ToolCallUpdate { tool_call_id, tool_kind, status, title, content, locations, raw_input, raw_output } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "tool_call_update", "toolCallId": tool_call_id, "kind": tool_kind, "status": status, "title": title, "content": content, "locations": locations, "rawInput": raw_input, "rawOutput": raw_output}}))),
        jockey::acp::AcpEvent::Plan { entries } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "plan", "entries": entries}}))),
        jockey::acp::AcpEvent::ModeUpdate { mode_id } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "current_mode_update", "currentModeId": mode_id}}))),
        jockey::acp::AcpEvent::ConfigUpdate { options } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "config_option_update", "configOptions": options}}))),
        jockey::acp::AcpEvent::SessionInfo { title } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "session_info_update", "title": title}}))),
        jockey::acp::AcpEvent::AvailableCommands { commands } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "available_commands_update", "availableCommands": commands}}))),
        // NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
        // The code is retained for future multi-user support but is currently unreachable.
        jockey::acp::AcpEvent::PermissionRequest { request_id, title, description, options } => {
            let permission_id = *next_permission_id;
            *next_permission_id += 1;
            permission_ids.insert(permission_id, request_id.clone());
            Some(rpc_request(permission_id, "session/request_permission", json!({
                "sessionId": session_id,
                "toolCall": { "toolCallId": request_id, "title": title, "kind": "other", "status": "pending", "rawInput": description.as_ref().map(|value| json!({"description": value})) },
                "options": options,
            })))
        }
        jockey::acp::AcpEvent::PermissionExpired { .. } => None,
        jockey::acp::AcpEvent::ReplayComplete => None,
        jockey::acp::AcpEvent::StatusUpdate { text } => Some(rpc_notification("_relay/stderr", json!({"data": text}))),
        jockey::acp::AcpEvent::AvailableModes { current, .. } => current.as_ref().map(|mode| rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "current_mode_update", "currentModeId": mode}}))),
    };
    match payload {
        Some(value) => send_json(sender, value).await,
        None => true,
    }
}

fn prompt_parts(request: &acp::PromptRequest) -> Result<(String, Vec<(String, String)>), String> {
    let mut prompt = Vec::new();
    let mut context = Vec::new();
    for (index, block) in request.prompt.iter().enumerate() {
        match block {
            acp::ContentBlock::Text(text) => prompt.push(text.text.clone()),
            acp::ContentBlock::ResourceLink(_) | acp::ContentBlock::Resource(_) => {
                context.push((
                    format!("attachment-{}", index + 1),
                    serde_json::to_string(block).map_err(|error| error.to_string())?,
                ));
            }
            acp::ContentBlock::Image(_) => {
                return Err("image prompt blocks are not supported by the relay".to_string());
            }
            acp::ContentBlock::Audio(_) => {
                return Err("audio prompt blocks are not supported by the relay".to_string());
            }
            _ => return Err("unsupported prompt block type".to_string()),
        }
    }
    let prompt = prompt.join("\n").trim().to_string();
    if prompt.is_empty() {
        return Err("prompt must contain at least one text block".to_string());
    }
    Ok((prompt, context))
}

fn parse_set_config_request(params: Value) -> Result<(String, String, String), String> {
    let session_id = params
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or("missing sessionId")?;
    let config_id = params
        .get("configId")
        .and_then(Value::as_str)
        .ok_or("missing configId")?;
    let value = match params.get("value") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => return Err("value must be a string or boolean".to_string()),
    };
    Ok((session_id.to_string(), config_id.to_string(), value))
}

fn update_config_options(
    options: &mut [Value],
    config_id: &str,
    value: &str,
) -> Option<ConfigCategory> {
    let mut matched_category = None;
    for option in options {
        if option.get("id").and_then(Value::as_str) != Some(config_id) {
            continue;
        }
        matched_category = config_option_category(option);
        patch_config_option_current_value(option, value);
    }
    matched_category
}

fn resolve_session_cwd(cwd: Option<PathBuf>, default_cwd: &str) -> Result<String, String> {
    match cwd {
        Some(cwd) if !cwd.is_absolute() => Err("cwd must be an absolute path".to_string()),
        Some(cwd) => Ok(cwd.to_string_lossy().into_owned()),
        None => {
            let default_path = Path::new(default_cwd);
            if default_path.is_absolute() {
                Ok(default_cwd.to_string())
            } else {
                Err("relay default cwd must be an absolute path".to_string())
            }
        }
    }
}

async fn reset_session_connection(
    relay: &RelayConfig,
    app_session_id: &str,
    request_id: Value,
) -> Result<(), Value> {
    let (reset_tx, reset_rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Reset {
            runtime_key: relay.adapter.runtime_key.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.to_string(),
            result_tx: reset_tx,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    reset_rx
        .await
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?
        .map_err(|error| rpc_error(request_id, -32000, error))
}

async fn prewarm_session(
    relay: &RelayConfig,
    app_session_id: &str,
    cwd: String,
    mcp_servers: Vec<acp::McpServer>,
    client_capabilities: acp::ClientCapabilities,
    resume_session_id: Option<String>,
    config_options: HashMap<String, Value>,
    request_id: Value,
) -> Result<(Vec<Value>, Vec<String>, String), Value> {
    let role_config_options = role_config_options(config_options)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Prewarm {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            client_capabilities,
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.to_string(),
            cwd,
            // Design decision: auto_approve is always true for the single-user desktop app.
            // The user is the operator — click-through permissions on every fs/write would
            // make pipelines unusable. When multi-user support is needed, this should be
            // configurable via relay settings.
            auto_approve: true,
            mcp_servers,
            role_mode: None,
            role_config_options,
            delta_tx: None,
            result_tx: Some(tx),
            resume_session_id,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    rx.await
        .map_err(|_| rpc_error(request_id, -32000, "ACP worker disconnected"))
}

fn role_config_options(config_options: HashMap<String, Value>) -> Result<Vec<Value>, String> {
    config_options
        .into_iter()
        .map(|(key, value)| normalize_role_config_option(&key, value))
        .collect()
}

fn config_value_to_json(key: &str, value: Value) -> Result<Value, String> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => Ok(value),
        Value::Null => Err(format!("configOptions.{key} must not be null")),
        other => Err(format!(
            "configOptions.{key} must be a scalar (string, bool, or number), got {}",
            match &other {
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "unknown",
            }
        )),
    }
}

fn normalize_role_config_option(key: &str, value: Value) -> Result<Value, String> {
    match value {
        Value::Object(mut option) => {
            if let Some(id) = option.get("id").and_then(Value::as_str) {
                if id != key {
                    return Err(format!(
                        "configOptions.{key}.id must match the config option key"
                    ));
                }
            } else {
                option.insert("id".to_string(), Value::String(key.to_string()));
            }

            option
                .entry("name".to_string())
                .or_insert_with(|| Value::String(key.to_string()));

            if !option.contains_key("currentValue") {
                return Err(format!(
                    "configOptions.{key} objects must include currentValue"
                ));
            }

            Ok(Value::Object(option))
        }
        other => {
            let current_value = config_value_to_json(key, other)?;
            let option_name = current_value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| current_value.to_string());
            let mut option = serde_json::Map::from_iter([
                ("id".to_string(), Value::String(key.to_string())),
                ("name".to_string(), Value::String(key.to_string())),
                ("currentValue".to_string(), current_value.clone()),
            ]);
            if current_value.is_boolean() {
                option.insert("type".to_string(), Value::String("boolean".to_string()));
            } else {
                option.insert("type".to_string(), Value::String("select".to_string()));
                option.insert(
                    "options".to_string(),
                    Value::Array(vec![json!({
                        "value": current_value,
                        "name": option_name,
                    })]),
                );
            }
            Ok(Value::Object(option))
        }
    }
}

fn patch_config_option_current_value(option: &mut Value, value: &str) {
    let next_value = match config_option_type(option) {
        Some("boolean") => Value::Bool(value.eq_ignore_ascii_case("true")),
        _ => Value::String(value.to_string()),
    };
    if let Some(object) = option.as_object_mut() {
        object.insert("currentValue".to_string(), next_value);
    }
}

fn build_modes(modes: &[String], current_mode_id: &Option<String>) -> Option<Value> {
    let current = current_mode_id
        .as_deref()
        .or_else(|| modes.first().map(|mode| mode.as_str()));
    current.map(|current| {
        json!({
            "currentModeId": current,
            "availableModes": modes.iter().map(|mode| json!({"id": mode, "name": mode})).collect::<Vec<_>>(),
        })
    })
}

// NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
// The code is retained for future multi-user support but is currently unreachable.
fn cancel_permissions(permission_ids: HashMap<u64, String>) {
    for request_id in permission_ids.into_values() {
        jockey::acp::respond_to_permission(&request_id, acp::RequestPermissionOutcome::Cancelled);
    }
}

/// Load configured MCP servers (global + per-agent) from maenifold.json,
/// deduplicated by name (case-insensitive), and converted to ACP schema.
fn load_configured_mcp_servers(root: &Path, agent_name: &str) -> Vec<acp::McpServer> {
    let config = AgentsConfig::load(root);
    let agent_servers = config
        .agents
        .get(agent_name)
        .map(|entry| entry.mcp_servers.as_slice())
        .unwrap_or(&[]);

    let mut seen = HashSet::new();
    let mut servers = Vec::new();
    for entry in config.mcp_servers.iter().chain(agent_servers.iter()) {
        let name = entry.name().to_ascii_lowercase();
        if seen.insert(name) {
            servers.push(entry.to_acp());
        }
    }
    servers
}

/// Merge configured MCP servers with client-provided ones.
/// Configured servers take precedence — client-provided servers are only
/// added when no configured server with the same name exists.
fn merge_mcp_servers(
    configured: &[acp::McpServer],
    client_provided: Vec<acp::McpServer>,
) -> Vec<acp::McpServer> {
    let mut servers = configured.to_vec();
    let existing: HashSet<String> = servers
        .iter()
        .filter_map(mcp_server_name)
        .map(|name| name.to_ascii_lowercase())
        .collect();
    for server in client_provided {
        let dominated = mcp_server_name(&server)
            .map(|name| existing.contains(&name.to_ascii_lowercase()))
            .unwrap_or(false);
        if !dominated {
            servers.push(server);
        }
    }
    servers
}

fn mcp_server_name(server: &acp::McpServer) -> Option<&str> {
    match server {
        acp::McpServer::Http(s) => Some(s.name.as_str()),
        acp::McpServer::Sse(s) => Some(s.name.as_str()),
        acp::McpServer::Stdio(s) => Some(s.name.as_str()),
        _ => None,
    }
}

fn rpc_request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn rpc_notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

fn rpc_ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    payload: Value,
) -> bool {
    sender
        .send(Message::Text(payload.to_string().into()))
        .await
        .is_ok()
}
