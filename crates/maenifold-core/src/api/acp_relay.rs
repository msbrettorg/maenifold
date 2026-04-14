use crate::api::routes::{ApiError, ApiResult, ApiState};
use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use agent_client_protocol as acp;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

#[derive(Deserialize)]
pub struct RelayQuery {
    agent: Option<String>,
}

#[derive(Clone)]
struct RelayConfig {
    agent_name: String,
    adapter: jockey::acp::StdioAdapterSpec,
    default_cwd: String,
}

struct RelaySession {
    app_session_id: String,
    session_id: String,
    cwd: String,
    config_options: Vec<Value>,
    modes: Vec<String>,
}

struct PendingPrompt {
    request_id: Value,
    cancelled: bool,
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
    Ok(ws.on_upgrade(move |socket| {
        relay_socket(
            socket,
            RelayConfig {
                agent_name,
                adapter,
                default_cwd,
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
) -> bool {
    let text = match message {
        Message::Text(text) => text,
        Message::Close(_) => return false,
        _ => return true,
    };
    let Ok(payload) = serde_json::from_str::<Value>(&text) else { return true; };
    let Some(object) = payload.as_object() else { return true; };

    if let Some(id) = object.get("id").and_then(Value::as_u64) {
        if object.get("method").is_none() {
            if let Some(request_id) = permission_ids.remove(&id) {
                let outcome = object
                    .get("result")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<acp::RequestPermissionResponse>(value).ok())
                    .map(|response| response.outcome)
                    .unwrap_or(acp::RequestPermissionOutcome::Cancelled);
                jockey::acp::respond_to_permission(&request_id, outcome);
                return true;
            }
        }
    }

    let Some(method) = object.get("method").and_then(Value::as_str) else { return true; };
    let id = object.get("id").cloned();
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

    let response = match method {
        "initialize" if id.is_some() => Some(
            handle_initialize(relay, session, id.clone().unwrap())
                .await
                .map(|result| rpc_ok(id.unwrap(), result)),
        ),
        "session/new" if id.is_some() => Some(
            handle_session_new(relay, session, params, id.clone().unwrap())
                .map(|result| rpc_ok(id.unwrap(), result))
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
        _ if id.is_some() => Some(Err(rpc_error(id.unwrap(), -32601, format!("Method not found: {method}")))),
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
    request_id: Value,
) -> Result<Value, Value> {
    if session.is_none() {
        let app_session_id = format!("acp-relay:{}", uuid::Uuid::new_v4());
        let (tx, rx) = oneshot::channel();
        jockey::acp::worker_tx()
            .send(jockey::acp::WorkerMsg::Prewarm {
                runtime_key: relay.adapter.runtime_key.clone(),
                binary: relay.adapter.binary.clone(),
                args: relay.adapter.args.clone(),
                env: relay.adapter.env.clone(),
                role_name: relay.agent_name.clone(),
                app_session_id: app_session_id.clone(),
                cwd: relay.default_cwd.clone(),
                auto_approve: true,
                mcp_servers: vec![],
                role_mode: None,
                role_config_options: vec![],
                result_tx: Some(tx),
                resume_session_id: None,
            })
            .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
        let (config_options, modes, session_id) = rx
            .await
            .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?;
        *session = Some(RelaySession {
            app_session_id,
            session_id,
            cwd: relay.default_cwd.clone(),
            config_options,
            modes,
        });
    }

    Ok(json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": { "image": false, "audio": false, "embeddedContext": true },
            "sessionCapabilities": {},
            "_meta": { "maenifold/extensions": { "supportedAgentMethods": ["session/set_config_option"] } }
        },
        "authMethods": [],
        "agentInfo": { "name": relay.agent_name, "title": relay.agent_name, "version": "0.1.0" }
    }))
}

fn handle_session_new(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    _params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(request_id, -32600, "initialize must be called before session/new"));
    };
    Ok(json!({
        "sessionId": active.session_id,
        "configOptions": active.config_options,
        "availableCommands": jockey::acp::list_available_commands(
            &active.app_session_id,
            &relay.adapter.runtime_key,
            &relay.agent_name,
        ),
        "modes": build_modes(&active.modes),
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
        return Err(rpc_error(request_id, -32000, "session/prompt already in progress"));
    }
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(request_id, -32600, "initialize must be called before session/prompt"));
    };
    let request: acp::PromptRequest =
        serde_json::from_value(params).map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    if request.session_id.to_string() != active.session_id {
        return Err(rpc_error(request_id, -32602, "sessionId does not match active session"));
    }
    let (prompt, context) =
        prompt_parts(&request).map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    let (event_tx, event_rx) = mpsc::channel(64);
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
            auto_approve: true,
            mcp_servers: vec![],
            role_mode: None,
            role_config_options: vec![],
            resume_session_id: None,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    *pending_prompt = Some(PendingPrompt { request_id: request_id.clone(), cancelled: false });
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
    update_config_options(&mut active.config_options, &request.1, &request.2);
    Ok(json!({ "configOptions": active.config_options }))
}

fn handle_cancel(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    permission_ids: &mut HashMap<u64, String>,
    params: Value,
) {
    let Some(active) = session.as_ref() else { return; };
    let Ok(request) = serde_json::from_value::<acp::CancelNotification>(params) else { return; };
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
        jockey::acp::AcpEvent::AvailableModes { modes, .. } => {
            session.modes = modes.iter().filter_map(|mode| mode.get("id").and_then(Value::as_str).map(ToString::to_string)).collect();
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
        jockey::acp::AcpEvent::ThoughtDelta { text } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": text}}}))),
        jockey::acp::AcpEvent::ToolCall { tool_call_id, title, tool_kind, status, content, locations, raw_input, raw_output } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "tool_call", "toolCallId": tool_call_id, "title": title, "kind": tool_kind, "status": status, "content": content, "locations": locations, "rawInput": raw_input, "rawOutput": raw_output}}))),
        jockey::acp::AcpEvent::ToolCallUpdate { tool_call_id, tool_kind, status, title, content, locations, raw_input, raw_output } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "tool_call_update", "toolCallId": tool_call_id, "kind": tool_kind, "status": status, "title": title, "content": content, "locations": locations, "rawInput": raw_input, "rawOutput": raw_output}}))),
        jockey::acp::AcpEvent::Plan { entries } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "plan", "entries": entries}}))),
        jockey::acp::AcpEvent::ModeUpdate { mode_id } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "current_mode_update", "currentModeId": mode_id}}))),
        jockey::acp::AcpEvent::ConfigUpdate { options } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "config_option_update", "configOptions": options}}))),
        jockey::acp::AcpEvent::SessionInfo { title } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "session_info_update", "title": title}}))),
        jockey::acp::AcpEvent::AvailableCommands { commands } => Some(rpc_notification("session/update", json!({"sessionId": session_id, "update": {"sessionUpdate": "available_commands_update", "availableCommands": commands}}))),
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
                context.push((format!("attachment-{}", index + 1), serde_json::to_string(block).map_err(|error| error.to_string())?));
            }
            acp::ContentBlock::Image(_) => return Err("image prompt blocks are not supported by the relay".to_string()),
            acp::ContentBlock::Audio(_) => return Err("audio prompt blocks are not supported by the relay".to_string()),
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
    let session_id = params.get("sessionId").and_then(Value::as_str).ok_or("missing sessionId")?;
    let config_id = params.get("configId").and_then(Value::as_str).ok_or("missing configId")?;
    let value = match params.get("value") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => return Err("value must be a string or boolean".to_string()),
    };
    Ok((session_id.to_string(), config_id.to_string(), value))
}

fn update_config_options(options: &mut [Value], config_id: &str, value: &str) {
    for option in options {
        if option.get("id").and_then(Value::as_str) != Some(config_id) {
            continue;
        }
        option["currentValue"] = match option.get("valueType").and_then(Value::as_str) {
            Some("boolean") => Value::Bool(value.eq_ignore_ascii_case("true")),
            _ => Value::String(value.to_string()),
        };
    }
}

fn build_modes(modes: &[String]) -> Option<Value> {
    modes.first().map(|current| {
        json!({
            "currentModeId": current,
            "availableModes": modes.iter().map(|mode| json!({"id": mode, "name": mode})).collect::<Vec<_>>(),
        })
    })
}

fn cancel_permissions(permission_ids: HashMap<u64, String>) {
    for request_id in permission_ids.into_values() {
        jockey::acp::respond_to_permission(&request_id, acp::RequestPermissionOutcome::Cancelled);
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
    sender.send(Message::Text(payload.to_string().into())).await.is_ok()
}
