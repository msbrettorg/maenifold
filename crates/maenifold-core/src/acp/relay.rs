use super::config::{AgentEntry, AgentsConfig};
use super::manager::AcpManager;
use axum::Router;
use axum::extract::{
    Query, State,
    ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
};
use axum::response::IntoResponse;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

// ── Chrome-ACP proxy protocol messages ──────────────────────────────────

#[derive(Deserialize)]
struct ProxyMessage {
    r#type: String,
    #[serde(default)]
    payload: Value,
}

fn server_msg(r#type: &str, payload: Value) -> String {
    serde_json::to_string(&serde_json::json!({ "type": r#type, "payload": payload })).unwrap()
}

fn status_connected(name: &str) -> String {
    server_msg("status", serde_json::json!({
        "connected": true,
        "agentInfo": { "name": name, "version": "1.0" }
    }))
}

fn status_disconnected() -> String {
    server_msg("status", serde_json::json!({ "connected": false }))
}

fn session_created(session_id: &str) -> String {
    server_msg("session_created", serde_json::json!({ "sessionId": session_id }))
}

fn session_update_chunk(session_id: &str, text: &str) -> String {
    server_msg("session_update", serde_json::json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": text }
        }
    }))
}

fn prompt_complete() -> String {
    server_msg("prompt_complete", serde_json::json!({ "stopReason": "end_turn" }))
}

fn error_msg(msg: &str) -> String {
    server_msg("error", serde_json::json!({ "message": msg }))
}

// ── Worker communication ────────────────────────────────────────────────

enum WorkerCommand {
    Connect,
    NewSession { cwd: String },
    Prompt { content: Vec<Value> },
}

enum WorkerEvent {
    Status(String),
    SessionCreated(String),
    Chunk(String, String),
    PromptComplete,
    Error(String),
    UnexpectedExit,
}

// ── Routes ──────────────────────────────────────────────────────────────

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<RelayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, state))
}

pub struct RelayState {
    pub config: AgentsConfig,
    pub default_agent: Option<String>,
}

pub fn acp_routes(config: AgentsConfig) -> Router {
    let default_agent = config.agents.keys().next().cloned();
    let state = Arc::new(RelayState {
        config,
        default_agent,
    });
    Router::new()
        .route("/ws/acp", get(ws_handler))
        .with_state(state)
}

// ── Socket handler ──────────────────────────────────────────────────────

async fn handle_socket(mut socket: WebSocket, params: HashMap<String, String>, state: Arc<RelayState>) {
    // Bounded channels (capacity 256) to cap memory if the browser stalls or
    // the worker falls behind.  See FR-32-SEC / TC-SEC.3.
    let (command_tx, command_rx) = mpsc::channel::<WorkerCommand>(256);
    let (event_tx, mut event_rx) = mpsc::channel::<WorkerEvent>(256);

    // Use ?agent= query param, fall back to first available
    let agent_name = params.get("agent")
        .filter(|n| !n.is_empty() && state.config.agents.contains_key(n.as_str()))
        .cloned()
        .or_else(|| state.default_agent.clone())
        .unwrap_or_default();
    let agent_entry = match state.config.agents.get(&agent_name) {
        Some(entry) => entry.clone(),
        None => {
            let _ = socket.send(Message::Text(error_msg("No agents configured").into())).await;
            return;
        }
    };
    let cwd = crate::config::Config::current().maenifold_root;
    spawn_agent_worker(agent_name.clone(), agent_entry, cwd, command_rx, event_tx.clone());

    loop {
        tokio::select! {
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Text(payload))) => {
                        let msg: ProxyMessage = match serde_json::from_str(&payload) {
                            Ok(m) => m,
                            Err(e) => {
                                let _ = socket.send(Message::Text(error_msg(&format!("Malformed message: {e}")).into())).await;
                                continue;
                            }
                        };

                        match msg.r#type.as_str() {
                            "connect" => {
                                match command_tx.try_send(WorkerCommand::Connect) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        let _ = socket.send(Message::Text(
                                            error_msg("Server busy – command backlog full").into(),
                                        )).await;
                                        continue;
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                                }
                            }
                            "new_session" => {
                                let cwd = msg.payload.get("cwd")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(".")
                                    .to_string();
                                match command_tx.try_send(WorkerCommand::NewSession { cwd }) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        let _ = socket.send(Message::Text(
                                            error_msg("Server busy – command backlog full").into(),
                                        )).await;
                                        continue;
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                                }
                            }
                            "prompt" => {
                                let content = msg.payload.get("content")
                                    .and_then(|v| v.as_array())
                                    .cloned()
                                    .unwrap_or_default();
                                match command_tx.try_send(WorkerCommand::Prompt { content }) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        let _ = socket.send(Message::Text(
                                            error_msg("Server busy – command backlog full").into(),
                                        )).await;
                                        continue;
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                                }
                            }
                            "disconnect" => {
                                let _ = socket.send(Message::Text(status_disconnected().into())).await;
                                break;
                            }
                            other => {
                                let _ = socket.send(Message::Text(error_msg(&format!("Unsupported message type: {other}")).into())).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(p))) => { let _ = socket.send(Message::Pong(p)).await; }
                    _ => {}
                }
            }
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(WorkerEvent::Status(s)) => {
                        let _ = socket.send(Message::Text(s.into())).await;
                    }
                    Some(WorkerEvent::SessionCreated(sid)) => {
                        let _ = socket.send(Message::Text(session_created(&sid).into())).await;
                    }
                    Some(WorkerEvent::Chunk(sid, text)) => {
                        let _ = socket.send(Message::Text(session_update_chunk(&sid, &text).into())).await;
                    }
                    Some(WorkerEvent::PromptComplete) => {
                        let _ = socket.send(Message::Text(prompt_complete().into())).await;
                    }
                    Some(WorkerEvent::Error(e)) => {
                        let _ = socket.send(Message::Text(error_msg(&e).into())).await;
                    }
                    Some(WorkerEvent::UnexpectedExit) => {
                        let _ = socket.send(Message::Text(error_msg("Agent process exited unexpectedly").into())).await;
                        let _ = socket.send(Message::Close(Some(CloseFrame {
                            code: 1011,
                            reason: "Agent process exited unexpectedly".into(),
                        }))).await;
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}

// ── Agent worker thread ─────────────────────────────────────────────────

fn spawn_agent_worker(
    agent_name: String,
    agent_entry: AgentEntry,
    cwd: PathBuf,
    command_rx: mpsc::Receiver<WorkerCommand>,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                // Pre-runtime: no async context available, use try_send.
                let _ = event_tx.try_send(WorkerEvent::Error(format!("Runtime creation failed: {e}")));
                return;
            }
        };
        let local = tokio::task::LocalSet::new();
        runtime.block_on(local.run_until(run_agent_worker(
            agent_name, agent_entry, cwd, command_rx, event_tx,
        )));
    });
}

async fn run_agent_worker(
    agent_name: String,
    agent_entry: AgentEntry,
    cwd: PathBuf,
    mut command_rx: mpsc::Receiver<WorkerCommand>,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    let mut manager: Option<AcpManager> = None;
    let mut session_id = String::new();
    let mut text_rx: Option<mpsc::UnboundedReceiver<String>> = None;

    loop {
        if let Some(ref mut rx) = text_rx {
            tokio::select! {
                maybe_cmd = command_rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => handle_worker_command(cmd, &agent_name, &agent_entry, &cwd, &mut manager, &mut session_id, &mut text_rx, &event_tx).await,
                        None => break,
                    }
                }
                maybe_chunk = rx.recv() => {
                    match maybe_chunk {
                        Some(text) => {
                            let _ = event_tx.send(WorkerEvent::Chunk(session_id.clone(), text)).await;
                        }
                        None => {
                            let _ = event_tx.send(WorkerEvent::PromptComplete).await;
                            text_rx = None;
                        }
                    }
                }
            }
        } else {
            match command_rx.recv().await {
                Some(cmd) => handle_worker_command(cmd, &agent_name, &agent_entry, &cwd, &mut manager, &mut session_id, &mut text_rx, &event_tx).await,
                None => break,
            }
        }
    }
}

async fn handle_worker_command(
    cmd: WorkerCommand,
    agent_name: &str,
    agent_entry: &AgentEntry,
    cwd: &PathBuf,
    manager: &mut Option<AcpManager>,
    session_id: &mut String,
    text_rx: &mut Option<mpsc::UnboundedReceiver<String>>,
    event_tx: &mpsc::Sender<WorkerEvent>,
) {
    match cmd {
        WorkerCommand::Connect => {
            match AcpManager::spawn(
                &agent_entry.command,
                &agent_entry.args,
                &agent_entry.env,
                cwd,
            ).await {
                Ok(mut mgr) => {
                    *text_rx = Some(mgr.take_text_stream());
                    let _ = event_tx.send(WorkerEvent::Status(status_connected(agent_name))).await;
                    *manager = Some(mgr);
                }
                Err(e) => {
                    let _ = event_tx.send(WorkerEvent::Error(e.to_string())).await;
                }
            }
        }
        WorkerCommand::NewSession { cwd: _ } => {
            if let &mut Some(ref mgr) = manager {
                *session_id = mgr.session_id().to_string();
                let _ = event_tx.send(WorkerEvent::SessionCreated(session_id.clone())).await;
            } else {
                let _ = event_tx.send(WorkerEvent::Error("Agent not connected".into())).await;
            }
        }
        WorkerCommand::Prompt { content } => {
            if let &mut Some(ref mgr) = manager {
                let messages: Vec<String> = content.iter()
                    .filter_map(|block| {
                        block.get("text").and_then(|v| v.as_str()).map(|s| s.to_string())
                    })
                    .collect();
                if let Err(e) = mgr.prompt(messages).await {
                    let _ = event_tx.send(WorkerEvent::Error(e.to_string())).await;
                }
            } else {
                let _ = event_tx.send(WorkerEvent::Error("Agent not connected".into())).await;
            }
        }
    }
}
