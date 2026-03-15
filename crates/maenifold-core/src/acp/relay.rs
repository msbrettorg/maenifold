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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, MissedTickBehavior};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "prompt")]
    Prompt { messages: Vec<String> },
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "session")]
    Session { session_id: String },
    #[serde(rename = "chunk")]
    Chunk { content: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "done")]
    Done,
}

enum WorkerCommand {
    Prompt { messages: Vec<String> },
}

enum WorkerEvent {
    Message(ServerMessage),
    UnexpectedExit,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(config): State<Arc<AgentsConfig>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, config))
}

pub fn acp_routes(config: AgentsConfig) -> Router {
    Router::new()
        .route("/ws/acp", get(ws_handler))
        .with_state(Arc::new(config))
}

async fn handle_socket(
    mut socket: WebSocket,
    params: HashMap<String, String>,
    config: Arc<AgentsConfig>,
) {
    let Some(agent_name) = params.get("agent").filter(|name| !name.is_empty()) else {
        let _ = send_error(&mut socket, "Missing required 'agent' query parameter").await;
        let _ = socket.send(Message::Close(None)).await;
        return;
    };

    let Some(agent_entry) = config.agents.get(agent_name).cloned() else {
        let _ = send_error(&mut socket, &format!("Unknown ACP agent '{agent_name}'")).await;
        let _ = socket.send(Message::Close(None)).await;
        return;
    };

    let cwd = crate::config::Config::current().maenifold_root;

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    spawn_agent_worker(agent_entry, cwd, command_rx, event_tx);

    loop {
        tokio::select! {
            maybe_browser_message = socket.recv() => {
                match maybe_browser_message {
                    Some(Ok(Message::Text(payload))) => {
                        if handle_browser_message(&mut socket, &command_tx, &payload).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {
                        if send_error(&mut socket, "Binary WebSocket messages are not supported")
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(Err(error)) => {
                        let _ =
                            send_error(&mut socket, &format!("WebSocket receive failed: {error}"))
                                .await;
                        break;
                    }
                }
            }
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(WorkerEvent::Message(message)) => {
                        if send_message(&mut socket, &message).await.is_err() {
                            break;
                        }
                    }
                    Some(WorkerEvent::UnexpectedExit) => {
                        let _ = close_unexpected_agent_exit(&mut socket).await;
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}

fn spawn_agent_worker(
    agent_entry: AgentEntry,
    cwd: PathBuf,
    command_rx: mpsc::UnboundedReceiver<WorkerCommand>,
    event_tx: mpsc::UnboundedSender<WorkerEvent>,
) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Error {
                    message: format!("Failed to create ACP relay runtime: {error}"),
                }));
                return;
            }
        };

        let local_set = tokio::task::LocalSet::new();
        runtime.block_on(local_set.run_until(run_agent_worker(
            agent_entry,
            cwd,
            command_rx,
            event_tx,
        )));
    });
}

async fn run_agent_worker(
    agent_entry: AgentEntry,
    cwd: PathBuf,
    mut command_rx: mpsc::UnboundedReceiver<WorkerCommand>,
    event_tx: mpsc::UnboundedSender<WorkerEvent>,
) {
    let mut manager = match AcpManager::spawn(
        &agent_entry.command,
        &agent_entry.args,
        &agent_entry.env,
        &cwd,
    )
    .await
    {
        Ok(manager) => manager,
        Err(error) => {
            let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Error {
                message: error.to_string(),
            }));
            return;
        }
    };

    let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Session {
        session_id: manager.session_id().to_string(),
    }));

    let mut text_stream = manager.take_text_stream();
    let mut exit_check = time::interval(Duration::from_millis(250));
    exit_check.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            maybe_command = command_rx.recv() => {
                match maybe_command {
                    Some(WorkerCommand::Prompt { messages }) => {
                        if let Err(error) = manager.prompt(messages).await {
                            let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Error {
                                message: error.to_string(),
                            }));
                        }
                    }
                    None => break,
                }
            }
            maybe_chunk = text_stream.recv() => {
                match maybe_chunk {
                    Some(content) => {
                        let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Chunk { content }));
                    }
                    None => {
                        match manager.try_wait() {
                            Ok(Some(_)) => {
                                let _ = event_tx.send(WorkerEvent::UnexpectedExit);
                            }
                            Ok(None) => {
                                let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Done));
                            }
                            Err(error) => {
                                let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Error {
                                    message: error.to_string(),
                                }));
                            }
                        }
                        break;
                    }
                }
            }
            _ = exit_check.tick() => {
                match manager.try_wait() {
                    Ok(Some(_)) => {
                        let _ = event_tx.send(WorkerEvent::UnexpectedExit);
                        break;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let _ = event_tx.send(WorkerEvent::Message(ServerMessage::Error {
                            message: error.to_string(),
                        }));
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_browser_message(
    socket: &mut WebSocket,
    command_tx: &mpsc::UnboundedSender<WorkerCommand>,
    payload: &str,
) -> Result<(), ()> {
    let message: ClientMessage = match serde_json::from_str(payload) {
        Ok(message) => message,
        Err(error) => {
            let _ = send_error(socket, &format!("Malformed relay message: {error}")).await;
            return Ok(());
        }
    };

    match message {
        ClientMessage::Prompt { messages } => {
            if command_tx.send(WorkerCommand::Prompt { messages }).is_err() {
                let _ = send_error(socket, "ACP session is no longer available").await;
                return Err(());
            }
        }
    }

    Ok(())
}

async fn send_message(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), ()> {
    let payload = serde_json::to_string(message).map_err(|_| ())?;
    socket
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}

async fn send_error(socket: &mut WebSocket, message: &str) -> Result<(), ()> {
    send_message(
        socket,
        &ServerMessage::Error {
            message: message.to_string(),
        },
    )
    .await
}

async fn close_unexpected_agent_exit(socket: &mut WebSocket) -> Result<(), ()> {
    send_error(socket, "Agent process exited unexpectedly").await?;
    socket
        .send(Message::Close(Some(CloseFrame {
            code: 1011,
            reason: "Agent process exited unexpectedly".into(),
        })))
        .await
        .map_err(|_| ())
}
