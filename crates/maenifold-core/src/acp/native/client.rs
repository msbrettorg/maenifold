/// ACP headless client — implements `agent_client_protocol::Client` for headless orchestration.
/// Accumulates session updates (text, thoughts, tool calls) per session.
use agent_client_protocol as acp;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::accumulator::SessionAccumulator;

pub(super) struct HeadlessClient {
    pub(super) cwd: PathBuf,
    pub(super) sessions: Arc<Mutex<HashMap<String, SessionAccumulator>>>,
}

impl HeadlessClient {
    /// Resolve and jail a path within the cwd. Rejects parent-dir traversal
    /// and ensures the canonical result stays under `self.cwd`.
    fn resolve_and_jail(&self, path: PathBuf) -> std::result::Result<PathBuf, acp::Error> {
        let resolved = if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        };
        if resolved
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(acp::Error::new(
                -32602,
                format!("path contains parent traversal: {}", resolved.display()),
            ));
        }
        let canonical_cwd = canonicalize_or_original(&self.cwd);
        let canonical_path = canonicalize_or_original(&resolved);
        if !canonical_path.starts_with(&canonical_cwd) {
            return Err(acp::Error::new(
                -32602,
                format!("path escapes working directory: {}", resolved.display()),
            ));
        }
        Ok(resolved)
    }

}

fn canonicalize_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[async_trait::async_trait(?Send)]
impl acp::Client for HeadlessClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        let session_id = args.session_id.to_string();
        if !self.sessions.lock().await.contains_key(&session_id) {
            tracing::info!(
                session_id = %session_id,
                "Permission request cancelled - session already cancelled"
            );
            return Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Cancelled,
            ));
        }
        let selected_option = args
            .options
            .iter()
            .find(|option| {
                matches!(
                    option.kind,
                    acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways
                )
            })
            .or_else(|| args.options.first())
            .cloned();
        let tool = args.tool_call.fields.title.as_deref().unwrap_or("unknown");
        tracing::info!(
            session_id = %args.session_id,
            permission_kind = ?selected_option.as_ref().map(|option| option.kind),
            tool,
            "Auto-granting ACP permission in headless mode"
        );
        let option_id = selected_option
            .map(|option| option.option_id)
            .unwrap_or_else(|| acp::PermissionOptionId::new("allow"));
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(option_id)),
        ))
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        let path = self.resolve_and_jail(args.path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                acp::Error::new(
                    -32603,
                    format!(
                        "failed to create parent directories for {}: {error}",
                        path.display()
                    ),
                )
            })?;
        }
        std::fs::write(&path, args.content).map_err(|error| {
            acp::Error::new(
                -32603,
                format!("failed to write {}: {error}", path.display()),
            )
        })?;
        Ok(acp::WriteTextFileResponse::new())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let path = self.resolve_and_jail(args.path)?;
        let content = std::fs::read_to_string(&path).map_err(|error| {
            acp::Error::new(
                -32603,
                format!("failed to read {}: {error}", path.display()),
            )
        })?;

        let start_line = match args.line {
            Some(0) => return Err(acp::Error::new(-32602, "line must be 1-based")),
            Some(line) => line as usize - 1,
            None => 0,
        };
        let limit = args.limit.map(|limit| limit as usize);

        let content = if start_line == 0 && limit.is_none() {
            content
        } else {
            content
                .split_inclusive('\n')
                .skip(start_line)
                .take(limit.unwrap_or(usize::MAX))
                .collect()
        };

        Ok(acp::ReadTextFileResponse::new(content))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        let session_id = args.session_id.to_string();
        match args.update {
            acp::SessionUpdate::UserMessageChunk(chunk) => {
                tracing::debug!(session_id = %session_id, chunk = ?chunk, "Received ACP user message chunk");
            }
            acp::SessionUpdate::AgentMessageChunk(chunk) => match chunk.content {
                acp::ContentBlock::Text(text) => {
                    let mut sessions = self.sessions.lock().await;
                    sessions
                        .entry(session_id.clone())
                        .or_default()
                        .append_text(&text.text);
                }
                content => {
                    tracing::debug!(
                        session_id = %session_id,
                        content = ?content,
                        "Received non-text ACP agent message chunk"
                    );
                }
            },
            acp::SessionUpdate::AgentThoughtChunk(chunk) => match chunk.content {
                acp::ContentBlock::Text(text) => {
                    let mut sessions = self.sessions.lock().await;
                    sessions
                        .entry(session_id.clone())
                        .or_default()
                        .append_thought(&text.text);
                }
                content => {
                    tracing::debug!(
                        session_id = %session_id,
                        content = ?content,
                        "Received non-text ACP thought chunk"
                    );
                }
            },
            acp::SessionUpdate::ToolCall(tool_call) => {
                let tool_call_id = tool_call.tool_call_id.to_string();
                let mut sessions = self.sessions.lock().await;
                sessions
                    .entry(session_id.clone())
                    .or_default()
                    .push_tool_call(tool_call);
                tracing::debug!(
                    session_id = %session_id,
                    tool_call_id = %tool_call_id,
                    "Recorded ACP tool call"
                );
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                let tool_call_id = update.tool_call_id.to_string();
                let apply_result = {
                    let mut sessions = self.sessions.lock().await;
                    Some(
                        sessions
                            .entry(session_id.clone())
                            .or_default()
                            .apply_tool_call_update(update),
                    )
                };
                if let Some(Err(error)) = apply_result {
                    tracing::warn!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        error = %error,
                        "Failed to reconstruct ACP tool call from update"
                    );
                }
                tracing::debug!(
                    session_id = %session_id,
                    tool_call_id = %tool_call_id,
                    "Processed ACP tool call update"
                );
            }
            acp::SessionUpdate::Plan(plan) => {
                tracing::debug!(session_id = %session_id, plan = ?plan, "Received ACP plan update");
            }
            acp::SessionUpdate::AvailableCommandsUpdate(update) => {
                tracing::info!(
                    session_id = %session_id,
                    command_count = update.available_commands.len(),
                    "Received ACP available commands update"
                );
            }
            acp::SessionUpdate::CurrentModeUpdate(update) => {
                tracing::debug!(session_id = %session_id, update = ?update, "Received ACP current mode update");
            }
            acp::SessionUpdate::ConfigOptionUpdate(update) => {
                tracing::debug!(session_id = %session_id, update = ?update, "Received ACP config option update");
            }
            acp::SessionUpdate::SessionInfoUpdate(update) => {
                tracing::debug!(session_id = %session_id, update = ?update, "Received ACP session info update");
            }
            acp::SessionUpdate::UsageUpdate(update) => {
                tracing::info!(
                    session_id = %session_id,
                    used = update.used,
                    size = update.size,
                    cost = ?update.cost,
                    "ACP usage update"
                );
            }
            _ => {
                tracing::debug!(session_id = %session_id, "Received unhandled ACP session update");
            }
        }
        Ok(())
    }
}
