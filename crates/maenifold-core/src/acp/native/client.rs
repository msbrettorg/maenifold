use std::{
    collections::HashMap,
    io::{ErrorKind, Write, stderr},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use sacp::schema as sacps;
use tokio::sync::Mutex;

use super::{accumulator::SessionAccumulator, types::AgentResponse};

#[derive(Default)]
struct SessionState {
    turn: Option<SessionAccumulator>,
}

#[derive(Clone)]
pub(super) struct HeadlessClient {
    cwd: PathBuf,
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    auto_grant_permissions: bool,
}

impl HeadlessClient {
    pub(super) fn new(cwd: PathBuf, auto_grant_permissions: bool) -> Self {
        Self {
            cwd,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            auto_grant_permissions,
        }
    }

    pub(super) async fn register_session(&self, session_id: impl Into<String>) {
        self.sessions
            .lock()
            .await
            .entry(session_id.into())
            .or_default();
    }

    pub(super) async fn remove_session(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }

    #[cfg(test)]
    pub(super) async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    pub(super) async fn begin_turn(&self, session_id: &str) {
        self.sessions
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .turn = Some(SessionAccumulator::default());
    }

    pub(super) async fn finish_turn(&self, session_id: &str) -> AgentResponse {
        self.sessions
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .turn
            .take()
            .unwrap_or_default()
            .into_response()
    }

    pub(super) async fn mark_cancelled(&self, session_id: &str) {
        if let Some(turn) = self
            .sessions
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .turn
            .as_mut()
        {
            turn.mark_in_progress_cancelled();
        }
    }

    fn resolve_path(&self, path: PathBuf) -> std::result::Result<PathBuf, sacp::Error> {
        let resolved = if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        };

        if resolved
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(sacp::Error::invalid_params().data(format!(
                "path contains parent traversal: {}",
                resolved.display()
            )));
        }

        Ok(resolved)
    }

    fn resolve_and_jail_existing(
        &self,
        path: PathBuf,
    ) -> std::result::Result<PathBuf, sacp::Error> {
        let resolved = self.resolve_path(path)?;
        let canonical_cwd = canonicalize_or_error(&self.cwd, "working directory")?;
        let canonical_path = canonicalize_or_error(&resolved, "path")?;
        if !canonical_path.starts_with(&canonical_cwd) {
            return Err(sacp::Error::invalid_params().data(format!(
                "path escapes working directory: {}",
                resolved.display()
            )));
        }

        Ok(resolved)
    }

    fn resolve_and_jail_for_write(
        &self,
        path: PathBuf,
    ) -> std::result::Result<PathBuf, sacp::Error> {
        let resolved = self.resolve_path(path)?;
        let canonical_cwd = canonicalize_or_error(&self.cwd, "working directory")?;
        let existing_ancestor = nearest_existing_ancestor(&resolved)?;
        let canonical_ancestor = canonicalize_or_error(&existing_ancestor, "path ancestor")?;
        if !canonical_ancestor.starts_with(&canonical_cwd) {
            return Err(sacp::Error::invalid_params().data(format!(
                "path escapes working directory: {}",
                resolved.display()
            )));
        }

        Ok(resolved)
    }

    pub(super) async fn request_permission(
        &self,
        args: sacps::RequestPermissionRequest,
    ) -> std::result::Result<sacps::RequestPermissionResponse, sacp::Error> {
        let session_id = args.session_id.to_string();
        if !self.sessions.lock().await.contains_key(&session_id) {
            return Ok(sacps::RequestPermissionResponse::new(
                sacps::RequestPermissionOutcome::Cancelled,
            ));
        }

        let option_id = if self.auto_grant_permissions {
            args.options.first().map(|option| option.option_id.clone())
        } else {
            args.options.iter().find_map(|option| {
                matches!(
                    option.kind,
                    sacps::PermissionOptionKind::RejectOnce
                        | sacps::PermissionOptionKind::RejectAlways
                )
                .then(|| option.option_id.clone())
            })
        }
        .ok_or_else(sacp::Error::method_not_found)?;

        Ok(sacps::RequestPermissionResponse::new(
            sacps::RequestPermissionOutcome::Selected(sacps::SelectedPermissionOutcome::new(
                option_id,
            )),
        ))
    }

    pub(super) async fn write_text_file(
        &self,
        args: sacps::WriteTextFileRequest,
    ) -> std::result::Result<sacps::WriteTextFileResponse, sacp::Error> {
        let path = self.resolve_and_jail_for_write(args.path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                sacp::Error::internal_error().data(format!(
                    "failed to create parent directories for {}: {error}",
                    path.display()
                ))
            })?;
            let canonical_cwd = canonicalize_or_error(&self.cwd, "working directory")?;
            let canonical_parent = canonicalize_or_error(parent, "write parent directory")?;
            if !canonical_parent.starts_with(&canonical_cwd) {
                return Err(sacp::Error::invalid_params().data(format!(
                    "path escapes working directory after directory creation: {}",
                    path.display()
                )));
            }
        }
        std::fs::write(&path, args.content).map_err(|error| {
            sacp::Error::internal_error()
                .data(format!("failed to write {}: {error}", path.display()))
        })?;
        Ok(sacps::WriteTextFileResponse::new())
    }

    pub(super) async fn read_text_file(
        &self,
        args: sacps::ReadTextFileRequest,
    ) -> std::result::Result<sacps::ReadTextFileResponse, sacp::Error> {
        let path = self.resolve_and_jail_existing(args.path)?;
        let content = std::fs::read_to_string(&path).map_err(|error| {
            sacp::Error::internal_error()
                .data(format!("failed to read {}: {error}", path.display()))
        })?;

        let start_line = match args.line {
            Some(0) => return Err(sacp::Error::invalid_params().data("line must be 1-based")),
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

        Ok(sacps::ReadTextFileResponse::new(content))
    }

    pub(super) async fn session_notification(
        &self,
        args: sacps::SessionNotification,
    ) -> std::result::Result<(), sacp::Error> {
        let session_id = args.session_id.to_string();
        let update_type = match &args.update {
            sacps::SessionUpdate::UserMessageChunk(_) => "user_message_chunk",
            sacps::SessionUpdate::AgentMessageChunk(_) => "agent_message_chunk",
            sacps::SessionUpdate::AgentThoughtChunk(_) => "agent_thought_chunk",
            sacps::SessionUpdate::ToolCall(_) => "tool_call",
            sacps::SessionUpdate::ToolCallUpdate(_) => "tool_call_update",
            sacps::SessionUpdate::Plan(_) => "plan",
            sacps::SessionUpdate::AvailableCommandsUpdate(_) => "available_commands_update",
            sacps::SessionUpdate::CurrentModeUpdate(_) => "current_mode_update",
            sacps::SessionUpdate::ConfigOptionUpdate(_) => "config_option_update",
            sacps::SessionUpdate::SessionInfoUpdate(_) => "session_info_update",
            _ => "unknown",
        };

        let mut sessions = self.sessions.lock().await;
        let state = sessions.entry(session_id.clone()).or_default();

        // T-93.5: Broadcast pipeline event BEFORE the accumulator guard so events
        // are emitted even when no active turn exists (e.g. session setup notifications).
        let pipeline_event = match &args.update {
            sacps::SessionUpdate::AgentMessageChunk(chunk) => {
                if let sacps::ContentBlock::Text(text) = &chunk.content {
                    Some(
                        crate::workflow::pipeline::stream::PipelineEvent::AgentMessage {
                            session_id: session_id.clone(),
                            text: text.text.clone(),
                        },
                    )
                } else {
                    None
                }
            }
            sacps::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let sacps::ContentBlock::Text(text) = &chunk.content {
                    Some(
                        crate::workflow::pipeline::stream::PipelineEvent::ThoughtChunk {
                            session_id: session_id.clone(),
                            text: text.text.clone(),
                        },
                    )
                } else {
                    None
                }
            }
            sacps::SessionUpdate::ToolCall(tc) => {
                Some(crate::workflow::pipeline::stream::PipelineEvent::ToolCall {
                    session_id: session_id.clone(),
                    tool_name: tc.title.clone(),
                })
            }
            sacps::SessionUpdate::ToolCallUpdate(tcu) => Some(
                crate::workflow::pipeline::stream::PipelineEvent::ToolCallUpdate {
                    session_id: session_id.clone(),
                    tool_name: tcu.tool_call_id.to_string(),
                },
            ),
            _ => None,
        };
        if let Some(event) = pipeline_event {
            let _ = crate::workflow::pipeline::stream::sender().send(event);
        }

        let Some(accumulator) = state.turn.as_mut() else {
            tracing::trace!(session_id = %session_id, update = update_type, "Ignoring session update without active turn");
            return Ok(());
        };

        match args.update {
            sacps::SessionUpdate::UserMessageChunk(_) => {}
            sacps::SessionUpdate::AgentMessageChunk(chunk) => {
                if let sacps::ContentBlock::Text(text) = chunk.content {
                    eprint!("{}", text.text);
                    let _ = stderr().flush();
                    accumulator.append_text(&text.text);
                }
            }
            sacps::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let sacps::ContentBlock::Text(text) = chunk.content {
                    accumulator.append_thought(&text.text);
                }
            }
            sacps::SessionUpdate::ToolCall(tool_call) => accumulator.push_tool_call(tool_call),
            sacps::SessionUpdate::ToolCallUpdate(update) => {
                accumulator.apply_tool_call_update(update)
            }
            _ => {
                tracing::debug!(session_id = %session_id, update = update_type, "Unhandled SessionUpdate variant");
            }
        }

        Ok(())
    }
}

fn canonicalize_or_error(path: &Path, context: &str) -> std::result::Result<PathBuf, sacp::Error> {
    path.canonicalize().map_err(|error| {
        sacp::Error::internal_error().data(format!(
            "failed to canonicalize {context} {}: {error}",
            path.display()
        ))
    })
}

fn nearest_existing_ancestor(path: &Path) -> std::result::Result<PathBuf, sacp::Error> {
    let mut current = path;
    loop {
        match std::fs::symlink_metadata(current) {
            Ok(_) => return Ok(current.to_path_buf()),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                current = current.parent().ok_or_else(|| {
                    sacp::Error::internal_error().data(format!(
                        "failed to find existing ancestor for {}",
                        path.display()
                    ))
                })?;
            }
            Err(error) => {
                return Err(sacp::Error::internal_error().data(format!(
                    "failed to inspect path {}: {error}",
                    current.display()
                )));
            }
        }
    }
}
