use super::config::{AgentEntry, AgentsConfig};
use crate::error::{MaenifoldError, Result};
use axum::Router;
use axum::extract::{
    Query, State,
    ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
};
use axum::response::IntoResponse;
use axum::routing::get;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<RelayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, state))
}

pub struct RelayState {
    pub config: AgentsConfig,
}

pub fn acp_routes(config: AgentsConfig) -> Router {
    let state = Arc::new(RelayState { config });
    Router::new()
        .route("/ws/acp", get(ws_handler))
        .with_state(state)
}

async fn handle_socket(
    mut socket: WebSocket,
    params: HashMap<String, String>,
    state: Arc<RelayState>,
) {
    let (agent_name, agent_entry) = match resolve_agent(&params, &state.config) {
        Ok(selection) => selection,
        Err(message) => {
            let _ = socket.send(Message::Text(error_msg(&message).into())).await;
            let _ = socket
                .send(Message::Close(Some(close_frame(1008, &message))))
                .await;
            return;
        }
    };

    let cwd = crate::config::Config::current().maenifold_root;
    let mut process = match RelayProcess::spawn(&agent_name, &agent_entry, &cwd).await {
        Ok(process) => process,
        Err(error) => {
            let message = error.to_string();
            let _ = socket.send(Message::Text(error_msg(&message).into())).await;
            let _ = socket
                .send(Message::Close(Some(close_frame(
                    1011,
                    "Failed to spawn agent process",
                ))))
                .await;
            return;
        }
    };

    let mut stdout_closed = false;
    let mut stderr_closed = false;

    loop {
        tokio::select! {
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Text(payload))) => {
                        if let Err(error) = process.write_line(&payload).await {
                            let message = format!("Failed to write to agent stdin: {error}");
                            let _ = socket.send(Message::Text(error_msg(&message).into())).await;
                            let _ = socket
                                .send(Message::Close(Some(close_frame(1011, "Failed to write to agent stdin"))))
                                .await;
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {
                        let _ = socket
                            .send(Message::Close(Some(close_frame(1003, "Binary WebSocket messages are not supported"))))
                            .await;
                        break;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Err(_)) => break,
                }
            }
            stdout_line = process.stdout_lines.next_line(), if !stdout_closed => {
                match stdout_line {
                    Ok(Some(line)) => {
                        if socket.send(Message::Text(line.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => stdout_closed = true,
                    Err(error) => {
                        let message = format!("Failed to read agent stdout: {error}");
                        let _ = socket.send(Message::Text(error_msg(&message).into())).await;
                        let _ = socket
                            .send(Message::Close(Some(close_frame(1011, "Failed to read agent stdout"))))
                            .await;
                        break;
                    }
                }
            }
            stderr_line = process.stderr_lines.next_line(), if !stderr_closed => {
                match stderr_line {
                    Ok(Some(line)) => {
                        if socket.send(Message::Text(stderr_msg(&line).into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => stderr_closed = true,
                    Err(error) => {
                        let message = format!("Failed to read agent stderr: {error}");
                        let _ = socket.send(Message::Text(error_msg(&message).into())).await;
                        let _ = socket
                            .send(Message::Close(Some(close_frame(1011, "Failed to read agent stderr"))))
                            .await;
                        break;
                    }
                }
            }
            exit = process.child.wait() => {
                let reason = match exit {
                    Ok(status) => agent_exit_reason(status),
                    Err(error) => format!("Agent process wait failed: {error}"),
                };
                let _ = socket.send(Message::Close(Some(close_frame(1011, &reason)))).await;
                break;
            }
        }
    }

    process.terminate().await;
}

fn resolve_agent(
    params: &HashMap<String, String>,
    config: &AgentsConfig,
) -> std::result::Result<(String, AgentEntry), String> {
    let available_agents_msg = available_agents_message(config);
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

    Ok((agent_name, agent_entry))
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

fn agent_exit_reason(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("Agent process exited with status code {code}"),
        None => "Agent process exited".to_string(),
    }
}

struct RelayProcess {
    child: Child,
    stdin: ChildStdin,
    stdout_lines: Lines<BufReader<ChildStdout>>,
    stderr_lines: Lines<BufReader<ChildStderr>>,
}

impl RelayProcess {
    async fn spawn(agent_name: &str, agent_entry: &AgentEntry, cwd: &Path) -> Result<Self> {
        let mut command = Command::new(&agent_entry.command);
        command
            .args(&agent_entry.args)
            .envs(&agent_entry.env)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|error| {
            MaenifoldError::Message(format!(
                "failed to spawn agent subprocess '{agent_name}' ({}): {error}",
                agent_entry.command
            ))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            MaenifoldError::Message("Agent subprocess stdin pipe was not available".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            MaenifoldError::Message("Agent subprocess stdout pipe was not available".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            MaenifoldError::Message("Agent subprocess stderr pipe was not available".to_string())
        })?;

        Ok(Self {
            child,
            stdin,
            stdout_lines: BufReader::new(stdout).lines(),
            stderr_lines: BufReader::new(stderr).lines(),
        })
    }

    async fn write_line(&mut self, line: &str) -> Result<()> {
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn terminate(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.child.kill().await;
                let _ = self.child.wait().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{available_agents_message, error_msg, resolve_agent, stderr_msg};
    use crate::acp::config::{AgentEntry, AgentsConfig};
    use std::collections::HashMap;

    fn test_config() -> AgentsConfig {
        AgentsConfig {
            agents: HashMap::from([
                (
                    "zebra".to_string(),
                    AgentEntry {
                        command: "/bin/echo".to_string(),
                        args: Vec::new(),
                        env: HashMap::new(),
                    },
                ),
                (
                    "alpha".to_string(),
                    AgentEntry {
                        command: "/bin/cat".to_string(),
                        args: Vec::new(),
                        env: HashMap::new(),
                    },
                ),
            ]),
        }
    }

    #[test]
    fn relay_lists_available_agents_in_sorted_order() {
        assert_eq!(available_agents_message(&test_config()), "alpha, zebra");
    }

    #[test]
    fn relay_requires_agent_query_param() {
        let params = HashMap::new();
        let error = resolve_agent(&params, &test_config()).expect_err("missing agent should fail");
        assert!(error.contains("Use ?agent=<name> query parameter"));
        assert!(error.contains("alpha, zebra"));
    }

    #[test]
    fn relay_rejects_unknown_agents_with_available_list() {
        let params = HashMap::from([("agent".to_string(), "unknown".to_string())]);
        let error = resolve_agent(&params, &test_config()).expect_err("unknown agent should fail");
        assert!(error.contains("Unknown agent 'unknown'"));
        assert!(error.contains("alpha, zebra"));
    }

    #[test]
    fn relay_formats_stderr_progress_messages() {
        let payload = stderr_msg("starting agent");
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("stderr payload should be valid json");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["method"], "_relay/stderr");
        assert_eq!(value["params"]["data"], "starting agent");
        assert!(value.get("id").is_none());
    }

    #[test]
    fn relay_formats_error_messages_as_json_rpc_notifications() {
        let payload = error_msg("agent failed");
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("error payload should be valid json");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["method"], "_relay/error");
        assert_eq!(value["params"]["message"], "agent failed");
        assert!(value.get("id").is_none());
    }
}
