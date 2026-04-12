use crate::acp_config::{AgentEntry, AgentsConfig};
use crate::acp_native::{AgentHandle, GLOBAL_AGENT_POOL};
use agent_client_protocol_schema as acp_schema;
use axum::Router;
use axum::extract::{
    Query,
    ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use sacp::{UntypedMessage, schema as sacps};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, sleep};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let agent = match resolve_agent(&params) {
        Ok(selection) => selection,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    let keep_session_alive = match keep_session_alive(&params) {
        Ok(value) => value,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    ws.on_upgrade(move |socket| handle_socket(socket, agent, keep_session_alive))
        .into_response()
}

pub fn acp_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/api/acp/session", get(ws_handler))
}

async fn handle_socket(socket: WebSocket, agent: RelayAgent, keep_session_alive: bool) {
    let cwd = relay_server_cwd();
    let handle = GLOBAL_AGENT_POOL
        .get_or_create(
            &agent.name,
            &agent.entry.command,
            &cwd,
            agent.entry.env.clone(),
            false,
        )
        .await;

    let (mut sender, mut receiver) = socket.split();
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<OutboundEvent>(64);
    let writer_task = tokio::spawn(async move {
        while let Some(event) = outbound_rx.recv().await {
            let send_result = match event {
                OutboundEvent::Text(payload) => sender.send(Message::Text(payload.into())).await,
                OutboundEvent::Pong(payload) => sender.send(Message::Pong(payload.into())).await,
                OutboundEvent::Close { code, reason } => {
                    let _ = sender
                        .send(Message::Close(Some(close_frame(code, &reason))))
                        .await;
                    break;
                }
            };
            if send_result.is_err() {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(forward_stderr(
        handle.subscribe_stderr(),
        outbound_tx.clone(),
    ));
    let death_task = tokio::spawn(watch_agent_death(handle.clone(), outbound_tx.clone()));
    let mut relay_sessions = HashSet::new();

    loop {
        match receiver.next().await {
            Some(Ok(Message::Text(payload))) => {
                let _ = GLOBAL_AGENT_POOL.touch_handle(&handle).await;
                if handle_client_message(&payload, &handle, &cwd, &outbound_tx, &mut relay_sessions)
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Some(Ok(Message::Ping(payload))) => {
                if outbound_tx
                    .send(OutboundEvent::Pong(payload.to_vec()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Some(Ok(Message::Binary(_))) => {
                let _ = outbound_tx
                    .send(OutboundEvent::Close {
                        code: 1003,
                        reason: "Binary WebSocket messages are not supported".to_string(),
                    })
                    .await;
                break;
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(Message::Pong(_))) => {}
            Some(Err(_)) => break,
        }
    }

    death_task.abort();
    stderr_task.abort();

    for session_id in &relay_sessions {
        if !keep_session_alive {
            let _ = handle.close_session(session_id).await;
            handle.forget_session(session_id).await;
        }
        handle.unregister_relay_session(session_id).await;
    }

    if !keep_session_alive && !handle.has_tracked_sessions().await {
        let _ = GLOBAL_AGENT_POOL.shutdown_handle(&handle).await;
    }

    drop(outbound_tx);
    let _ = writer_task.await;
}

#[derive(Clone)]
struct RelayAgent {
    name: String,
    entry: AgentEntry,
}

enum OutboundEvent {
    Text(String),
    Pong(Vec<u8>),
    Close { code: u16, reason: String },
}

struct RelayError {
    code: i64,
    message: String,
}

impl RelayError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
        }
    }
}

fn resolve_agent(params: &HashMap<String, String>) -> std::result::Result<RelayAgent, String> {
    let cwd = crate::config::Config::current().maenifold_root;
    let config = AgentsConfig::load(&cwd);
    let available_agents_msg = available_agents_message(&config);
    let agent_name = match params.get("agent").map(|name| name.trim()) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => {
            return Err(format!(
                "No agent specified. Use ?agent=<name> query parameter. Available agents: {available_agents_msg}"
            ));
        }
    };

    let agent_entry = config.agents.get(&agent_name).cloned().ok_or_else(|| {
        format!("Unknown agent '{agent_name}'. Available agents: {available_agents_msg}")
    })?;

    Ok(RelayAgent {
        name: agent_name,
        entry: agent_entry,
    })
}

fn keep_session_alive(params: &HashMap<String, String>) -> std::result::Result<bool, String> {
    match params
        .get("keepSessionAlive")
        .or_else(|| params.get("keep_session_alive"))
        .map(|value| value.trim().to_ascii_lowercase())
    {
        None => Ok(true),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes") => Ok(true),
        Some(value) if matches!(value.as_str(), "0" | "false" | "no") => Ok(false),
        Some(value) => Err(format!(
            "Invalid keepSessionAlive query parameter '{value}'. Expected true or false."
        )),
    }
}

fn available_agents_message(config: &AgentsConfig) -> String {
    let mut available_agents = config.agents.keys().cloned().collect::<Vec<_>>();
    available_agents.sort();
    if available_agents.is_empty() {
        "none".to_string()
    } else {
        available_agents.join(", ")
    }
}

fn relay_server_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| crate::config::Config::current().maenifold_root)
}

fn enforce_server_cwd(request_cwd: &mut PathBuf, server_cwd: &Path, method: &str) {
    if request_cwd != server_cwd {
        tracing::warn!(
            method,
            client_cwd = %request_cwd.display(),
            server_cwd = %server_cwd.display(),
            "Ignoring client-supplied ACP cwd for relay filesystem scope"
        );
        *request_cwd = server_cwd.to_path_buf();
    }
}

fn error_msg(message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": "_relay/error",
        "params": { "message": message }
    }))
    .expect("error control messages must serialize")
}

fn stderr_msg(line: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": "_relay/stderr",
        "params": { "data": line }
    }))
    .expect("stderr control messages must serialize")
}

fn close_frame(code: u16, reason: &str) -> CloseFrame {
    CloseFrame {
        code,
        reason: reason.to_string().into(),
    }
}

async fn forward_stderr(
    mut stderr_rx: broadcast::Receiver<String>,
    outbound: mpsc::Sender<OutboundEvent>,
) {
    loop {
        match stderr_rx.recv().await {
            Ok(line) => {
                if outbound
                    .send(OutboundEvent::Text(stderr_msg(&line)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}

async fn watch_agent_death(handle: Arc<AgentHandle>, outbound: mpsc::Sender<OutboundEvent>) {
    loop {
        if !handle.is_alive() {
            let _ = outbound
                .send(OutboundEvent::Close {
                    code: 1011,
                    reason: "Agent process exited".to_string(),
                })
                .await;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn handle_client_message(
    payload: &str,
    handle: &Arc<AgentHandle>,
    server_cwd: &Path,
    outbound: &mpsc::Sender<OutboundEvent>,
    relay_sessions: &mut HashSet<String>,
) -> std::result::Result<(), ()> {
    let value: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(error) => {
            let _ = outbound
                .send(OutboundEvent::Text(error_msg(&format!(
                    "Outbound ACP frame must be valid JSON: {error}"
                ))))
                .await;
            return Ok(());
        }
    };
    let payload = serde_json::to_string(&value).unwrap_or_else(|_| payload.to_string());

    if value.get("result").is_some() || value.get("error").is_some() {
        handle_client_response(&value, handle).await;
        return Ok(());
    }

    let Some(method) = value.get("method").and_then(Value::as_str) else {
        let _ = outbound
            .send(OutboundEvent::Text(error_msg(
                "Outbound ACP frame must include a JSON-RPC method",
            )))
            .await;
        return Ok(());
    };

    let params = value.get("params").cloned().unwrap_or(Value::Null);
    match value.get("id").cloned() {
        Some(id) => {
            let message = match dispatch_request(
                method,
                params,
                handle,
                server_cwd,
                outbound,
                relay_sessions,
            )
            .await
            {
                Ok(result) => json_rpc_success(&id, result),
                Err(error) => json_rpc_error(&id, error.code, &error.message),
            };
            if outbound.send(OutboundEvent::Text(message)).await.is_err() {
                return Err(());
            }
        }
        None => {
            if let Err(error) = dispatch_notification(method, params, handle, relay_sessions).await
            {
                let _ = outbound
                    .send(OutboundEvent::Text(error_msg(&error.message)))
                    .await;
            }
        }
    }

    debug_assert!(!payload.contains('\n'));
    Ok(())
}

async fn handle_client_response(value: &Value, handle: &Arc<AgentHandle>) {
    let _ = GLOBAL_AGENT_POOL.touch_handle(handle).await;
    let request_id = value.get("id").and_then(Value::as_u64);
    let Some(request_id) = request_id else {
        return;
    };
    let payload = if let Some(result) = value.get("result") {
        Ok(result.clone())
    } else {
        let message = value
            .get("error")
            .and_then(Value::as_object)
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("relay response contained no result")
            .to_string();
        Err(message)
    };
    let _ = handle.resolve_relay_response(request_id, payload).await;
}

async fn dispatch_request(
    method: &str,
    params: Value,
    handle: &Arc<AgentHandle>,
    server_cwd: &Path,
    outbound: &mpsc::Sender<OutboundEvent>,
    relay_sessions: &mut HashSet<String>,
) -> std::result::Result<Value, RelayError> {
    match method {
        "initialize" => initialize_response(handle).await,
        "session/new" => {
            let mut request: sacps::NewSessionRequest =
                serde_json::from_value(params).map_err(invalid_params_error)?;
            enforce_server_cwd(&mut request.cwd, server_cwd, "session/new");
            let conn = ready_connection(handle).await?;
            let response = conn
                .send_request(request.clone())
                .block_task()
                .await
                .map_err(internal_acp_error)?;
            register_relay_session(
                handle,
                outbound,
                relay_sessions,
                response.session_id.to_string(),
            )
            .await;
            serde_json::to_value(response).map_err(internal_json_error)
        }
        "session/load" => {
            let mut request: sacps::LoadSessionRequest =
                serde_json::from_value(params).map_err(invalid_params_error)?;
            enforce_server_cwd(&mut request.cwd, server_cwd, "session/load");
            let conn = ready_connection(handle).await?;
            let response = conn
                .send_request(request.clone())
                .block_task()
                .await
                .map_err(internal_acp_error)?;
            register_relay_session(
                handle,
                outbound,
                relay_sessions,
                request.session_id.to_string(),
            )
            .await;
            serde_json::to_value(response).map_err(internal_json_error)
        }
        "session/close" => {
            let request: acp_schema::CloseSessionRequest =
                serde_json::from_value(params.clone()).map_err(invalid_params_error)?;
            let response = forward_request(method, params, handle).await?;
            let session_id = request.session_id.to_string();
            relay_sessions.remove(&session_id);
            handle.forget_session(&session_id).await;
            handle.unregister_relay_session(&session_id).await;
            Ok(response)
        }
        _ => forward_request(method, params, handle).await,
    }
}

async fn dispatch_notification(
    method: &str,
    params: Value,
    handle: &Arc<AgentHandle>,
    _relay_sessions: &mut HashSet<String>,
) -> std::result::Result<(), RelayError> {
    forward_notification(method, params, handle).await
}

async fn register_relay_session(
    handle: &Arc<AgentHandle>,
    outbound: &mpsc::Sender<OutboundEvent>,
    relay_sessions: &mut HashSet<String>,
    session_id: String,
) {
    if relay_sessions.insert(session_id.clone()) {
        handle.track_session(&session_id).await;
        handle
            .register_relay_session(
                &session_id,
                relay_outbound(handle.clone(), outbound.clone()),
            )
            .await;
    }
}

fn relay_outbound(
    handle: Arc<AgentHandle>,
    outbound: mpsc::Sender<OutboundEvent>,
) -> mpsc::Sender<String> {
    let (tx, mut rx) = mpsc::channel::<String>(64);
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = GLOBAL_AGENT_POOL.touch_handle(&handle).await;
            if outbound.send(OutboundEvent::Text(message)).await.is_err() {
                break;
            }
        }
    });
    tx
}

async fn initialize_response(handle: &Arc<AgentHandle>) -> std::result::Result<Value, RelayError> {
    let _ = ready_connection(handle).await?;
    Ok(json!({
        "protocolVersion": 1,
        "agentCapabilities": handle.capabilities(),
        "agentInfo": handle.agent_info(),
        "authMethods": handle.auth_methods().unwrap_or_default(),
    }))
}

async fn forward_request(
    method: &str,
    params: Value,
    handle: &Arc<AgentHandle>,
) -> std::result::Result<Value, RelayError> {
    let conn = ready_connection(handle).await?;
    let request = UntypedMessage::new(method, params)
        .map_err(|error| RelayError::internal(error.to_string()))?;
    conn.send_request(request)
        .block_task()
        .await
        .map_err(internal_acp_error)
}

async fn forward_notification(
    method: &str,
    params: Value,
    handle: &Arc<AgentHandle>,
) -> std::result::Result<(), RelayError> {
    let conn = ready_connection(handle).await?;
    let notification = UntypedMessage::new(method, params)
        .map_err(|error| RelayError::internal(error.to_string()))?;
    conn.send_notification(notification)
        .map_err(internal_acp_error)
}

async fn ready_connection(
    handle: &Arc<AgentHandle>,
) -> std::result::Result<sacp::ConnectionTo<sacp::Agent>, RelayError> {
    handle
        .wait_for_connection(Duration::from_secs(30))
        .await
        .map_err(|error| RelayError::internal(error.to_string()))
}

fn invalid_params_error(error: serde_json::Error) -> RelayError {
    RelayError::invalid_params(format!("Invalid ACP params: {error}"))
}

fn internal_json_error(error: serde_json::Error) -> RelayError {
    RelayError::internal(format!("Failed to serialize ACP payload: {error}"))
}

fn internal_acp_error(error: sacp::Error) -> RelayError {
    RelayError::internal(error.to_string())
}

fn json_rpc_success(id: &Value, result: Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .expect("success responses must serialize")
}

fn json_rpc_error(id: &Value, code: i64, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    }))
    .expect("error responses must serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_server_cwd_replaces_client_scope() {
        let server_cwd = PathBuf::from("/server-root");
        let mut request_cwd = PathBuf::from("/client-root");

        enforce_server_cwd(&mut request_cwd, &server_cwd, "session/new");

        assert_eq!(request_cwd, server_cwd);
    }
}
