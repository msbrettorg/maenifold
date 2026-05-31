use super::acp_relay::{ConfigCategory, config_option_category};
use agent_client_protocol::schema::{self as acp};
use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn config_option_type(option: &Value) -> Option<&str> {
    option.get("type").and_then(Value::as_str)
}

pub(super) fn update_config_options(
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

pub(super) fn resolve_session_cwd(
    cwd: Option<PathBuf>,
    default_cwd: &str,
) -> Result<String, String> {
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
pub(super) fn patch_config_option_current_value(option: &mut Value, value: &str) {
    let next_value = match config_option_type(option) {
        Some("boolean") => Value::Bool(value.eq_ignore_ascii_case("true")),
        _ => Value::String(value.to_string()),
    };
    if let Some(object) = option.as_object_mut() {
        object.insert("currentValue".to_string(), next_value);
    }
}

// NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
// The code is retained for future multi-user support but is currently unreachable.
pub(super) fn cancel_permissions(permission_ids: HashMap<u64, String>) {
    for request_id in permission_ids.into_values() {
        jockey::acp::respond_to_permission(&request_id, acp::RequestPermissionOutcome::Cancelled);
    }
}
pub(super) fn rpc_request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

pub(super) fn rpc_notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

pub(super) fn rpc_ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub(super) fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

pub(super) fn reject_client_mcp_servers(
    params: &Value,
    request_id: Value,
    method: &str,
) -> Result<(), Value> {
    let Some(mcp_servers) = params.get("mcpServers") else {
        return Ok(());
    };
    if mcp_servers.as_array().is_none_or(Vec::is_empty) {
        return Ok(());
    }
    Err(rpc_error(
        request_id,
        -32602,
        format!("{method} does not accept client-provided mcpServers"),
    ))
}

pub(super) async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    payload: Value,
) -> bool {
    sender
        .send(Message::Text(payload.to_string().into()))
        .await
        .is_ok()
}
