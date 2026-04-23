use super::acp_relay::{PendingPrompt, RelayConfig, RelaySession};
use super::acp_relay_helpers::{cancel_permissions, rpc_error, rpc_notification, rpc_ok, rpc_request, send_json};
use super::acp_relay_prompt::{handle_prompt, handle_set_config_option};
use super::acp_relay_session::{handle_initialize, handle_session_load, handle_session_new};
use agent_client_protocol as acp;
use axum::extract::ws::{Message, WebSocket};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

pub(super) async fn handle_incoming(
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
pub(super) fn handle_cancel(
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

pub(super) fn apply_event_state(session: &mut RelaySession, event: &jockey::acp::AcpEvent) {
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

pub(super) async fn forward_event(
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
