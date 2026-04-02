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
            return Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Cancelled,
            ));
        }

        let option_id = args
            .options
            .iter()
            .find(|option| {
                matches!(
                    option.kind,
                    acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways
                )
            })
            .or_else(|| args.options.first())
            .map(|option| option.option_id.clone())
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

    async fn create_terminal(
        &self,
        _args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal(
        &self,
        _args: acp::KillTerminalRequest,
    ) -> acp::Result<acp::KillTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        let session_id = args.session_id.to_string();
        let update_type = match &args.update {
            acp::SessionUpdate::UserMessageChunk(_) => "user_message_chunk",
            acp::SessionUpdate::AgentMessageChunk(_) => "agent_message_chunk",
            acp::SessionUpdate::AgentThoughtChunk(_) => "agent_thought_chunk",
            acp::SessionUpdate::ToolCall(_) => "tool_call",
            acp::SessionUpdate::ToolCallUpdate(_) => "tool_call_update",
            acp::SessionUpdate::Plan(_) => "plan",
            acp::SessionUpdate::AvailableCommandsUpdate(_) => "available_commands_update",
            acp::SessionUpdate::CurrentModeUpdate(_) => "current_mode_update",
            acp::SessionUpdate::ConfigOptionUpdate(_) => "config_option_update",
            acp::SessionUpdate::SessionInfoUpdate(_) => "session_info_update",
            _ => "unknown",
        };
        eprintln!("[ACP] session_notification: session={session_id} update={update_type}");

        let mut sessions = self.sessions.lock().await;
        let accumulator = sessions.entry(session_id.clone()).or_default();

        match args.update {
            acp::SessionUpdate::UserMessageChunk(_) => {}
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    accumulator.append_text(&text.text);
                }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    accumulator.append_thought(&text.text);
                }
            }
            acp::SessionUpdate::ToolCall(tool_call) => {
                accumulator.push_tool_call(tool_call);
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                if let Err(error) = accumulator.apply_tool_call_update(update) {
                    tracing::warn!(
                        session_id = %session_id,
                        "Failed to reconstruct ACP tool call from update: {error}"
                    );
                }
            }
            _ => {
                tracing::debug!(session_id = %session_id, update = update_type, "Unhandled SessionUpdate variant");
            }
        }

        Ok(())
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> acp::Result<()> {
        Err(acp::Error::method_not_found())
    }
}
