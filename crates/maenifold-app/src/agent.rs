use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(not(target_os = "windows"))]
use shell_escape;

use crate::config::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub agent_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStderr {
    pub agent_id: String,
    pub line: String,
}

struct RunningAgent {
    #[allow(dead_code)]
    child: Child,
    stdin: Arc<RwLock<std::process::ChildStdin>>,
}

pub struct AgentManager {
    agents: Arc<RwLock<HashMap<String, RunningAgent>>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn spawn_agent(
        &self,
        name: String,
        config: &AgentConfig,
        app_handle: AppHandle,
    ) -> Result<AgentInstance, String> {
        let agent_id = Uuid::new_v4().to_string();

        // On Windows, we need to use cmd.exe to properly resolve .cmd/.bat files like npx
        #[cfg(target_os = "windows")]
        let mut child = {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C")
                .arg(&config.command)
                .args(&config.args)
                .envs(&config.env)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000); // CREATE_NO_WINDOW
            cmd.spawn()
                .map_err(|e| format!("Failed to spawn agent: {}", e))?
        };

        #[cfg(not(target_os = "windows"))]
        let mut child = {
            use std::borrow::Cow;

            // Build shell command with proper quoting for command and arguments
            let escaped_command = shell_escape::escape(Cow::Borrowed(config.command.as_str()));
            let shell_command = if config.args.is_empty() {
                escaped_command.to_string()
            } else {
                let quoted_args: Vec<String> = config
                    .args
                    .iter()
                    .map(|arg| shell_escape::escape(Cow::Borrowed(arg.as_str())).to_string())
                    .collect();
                format!("{} {}", escaped_command, quoted_args.join(" "))
            };

            // Determine the shell to use for PATH resolution without invoking a login shell.
            let user_shell = std::env::var("SHELL").unwrap_or_default();
            let shell_name = std::path::Path::new(&user_shell)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            let shell = match shell_name {
                "bash" | "zsh" | "ksh" | "fish" => user_shell.as_str(),
                _ => {
                    // Probe for bash at common paths, fall back to /bin/sh (common default on Unix-like systems).
                    if std::path::Path::new("/bin/bash").exists() {
                        "/bin/bash"
                    } else if std::path::Path::new("/usr/bin/bash").exists() {
                        "/usr/bin/bash"
                    } else {
                        "/bin/sh"
                    }
                }
            };

            let mut cmd = Command::new(shell);
            cmd.arg("-c")
                .arg(&shell_command)
                .envs(&config.env)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn agent: {}", e))?
        };

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to get stdin".to_string())?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to get stdout".to_string())?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to get stderr".to_string())?;

        let stdin = Arc::new(RwLock::new(stdin));

        // Spawn a thread to read stdout and emit events
        let agent_id_clone = agent_id.clone();
        let app_handle_clone = app_handle.clone();
        let agents_clone = Arc::clone(&self.agents);

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(message) => {
                        let Some(message) =
                            normalize_inbound_frame(&message, &agent_id_clone, "stdout")
                        else {
                            continue;
                        };
                        let agent_message = AgentMessage {
                            agent_id: agent_id_clone.clone(),
                            message,
                        };
                        let _ = app_handle_clone.emit("agent-message", agent_message);
                    }
                    Err(_) => break,
                }
            }
            // Agent process ended, remove from map
            agents_clone.write().remove(&agent_id_clone);
            let _ = app_handle_clone.emit("agent-closed", agent_id_clone);
        });

        // Spawn a thread to read stderr and emit events (for startup progress)
        let agent_id_clone2 = agent_id.clone();
        let app_handle_clone2 = app_handle.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line_content) => {
                        let stderr_msg = AgentStderr {
                            agent_id: agent_id_clone2.clone(),
                            line: line_content,
                        };
                        let _ = app_handle_clone2.emit("agent-stderr", stderr_msg);
                    }
                    Err(_) => break,
                }
            }
        });

        let running_agent = RunningAgent { child, stdin };
        self.agents.write().insert(agent_id.clone(), running_agent);

        Ok(AgentInstance { id: agent_id, name })
    }

    pub fn send_message(&self, agent_id: &str, message: &str) -> Result<(), String> {
        let agents = self.agents.read();
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| format!("Agent not found: {}", agent_id))?;
        let message = validate_outbound_frame(message)?;

        let mut stdin = agent.stdin.write();
        debug_assert!(!message.contains('\n'));
        writeln!(stdin, "{}", message).map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin
            .flush()
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        Ok(())
    }

    pub fn kill_agent(&self, agent_id: &str) -> Result<(), String> {
        let mut agents = self.agents.write();
        if let Some(mut agent) = agents.remove(agent_id) {
            agent
                .child
                .kill()
                .map_err(|e| format!("Failed to kill agent: {}", e))?;
        }
        Ok(())
    }

    pub fn list_running_agents(&self) -> Vec<String> {
        self.agents.read().keys().cloned().collect()
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_outbound_frame(frame: &str) -> Result<String, String> {
    let compact = compact_json_frame(frame)
        .map_err(|error| format!("Outbound ACP frame must be valid JSON: {error}"))?;
    debug_assert!(!compact.contains('\n'));
    Ok(compact)
}

fn normalize_inbound_frame(frame: &str, agent_id: &str, source: &str) -> Option<String> {
    match compact_json_frame(frame) {
        Ok(compact) => Some(compact),
        Err(error) => {
            eprintln!(
                "Warning: skipping invalid ACP {source} frame from agent {agent_id}: {error}; raw={frame:?}"
            );
            None
        }
    }
}

fn compact_json_frame(frame: &str) -> serde_json::Result<String> {
    let value: Value = serde_json::from_str(frame)?;
    serde_json::to_string(&value)
}

#[cfg(test)]
mod tests {
    use super::{normalize_inbound_frame, validate_outbound_frame};

    #[test]
    fn agent_compacts_outbound_json_frames_before_writing() {
        let payload = "{\n  \"jsonrpc\": \"2.0\",\n  \"id\": 1,\n  \"method\": \"ping\"\n}";
        let compact = validate_outbound_frame(payload).expect("payload should be compacted");
        assert!(!compact.contains('\n'));
        let value: serde_json::Value =
            serde_json::from_str(&compact).expect("compacted payload should remain valid json");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 1);
        assert_eq!(value["method"], "ping");
    }

    #[test]
    fn agent_rejects_invalid_outbound_json_frames() {
        let error = validate_outbound_frame("{not json]").expect_err("invalid json should fail");
        assert!(error.contains("Outbound ACP frame must be valid JSON"));
    }

    #[test]
    fn agent_skips_invalid_inbound_stdout_frames() {
        assert!(normalize_inbound_frame("{not json]", "agent-1", "stdout").is_none());
    }
}
