use agent_client_protocol::{self as acp};
use serde_json::{Value, json};

pub(super) use super::client_terminal::shutdown_terminals;
use super::worker::{AcpEvent, DeltaSlot, permission_requests};

const RAW_PAYLOAD_LIMIT: usize = 64 * 1024;

fn cap_raw(v: Option<Value>) -> Option<Value> {
    let v = v?;
    let s = v.to_string();
    if s.len() <= RAW_PAYLOAD_LIMIT {
        Some(v)
    } else {
        Some(json!(format!("[truncated {} bytes]", s.len())))
    }
}

pub(super) struct JockeyUiClient {
    pub(super) delta_slot: DeltaSlot,
    pub(super) auto_approve: bool,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for JockeyUiClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        if self.auto_approve {
            let outcome = if let Some(opt) = args.options.first() {
                acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                    opt.option_id.clone(),
                ))
            } else {
                acp::RequestPermissionOutcome::Cancelled
            };
            return Ok(acp::RequestPermissionResponse::new(outcome));
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let event = AcpEvent::PermissionRequest {
            request_id: request_id.clone(),
            title: args.tool_call.fields.title.clone().unwrap_or_default(),
            description: None,
            options: args
                .options
                .iter()
                .map(|o| serde_json::to_value(o).unwrap_or(json!({})))
                .collect(),
        };
        if let Ok(guard) = self.delta_slot.lock() {
            if let Some(tx) = guard.as_ref() {
                if tx.try_send(event).is_err() {
                    use super::adapter::acp_log;
                    acp_log(
                        "permission.channel.drop",
                        serde_json::json!({ "requestId": request_id }),
                    );
                }
            }
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        permission_requests().insert(request_id.clone(), tx);
        match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
            Ok(Ok(decision)) => Ok(acp::RequestPermissionResponse::new(decision)),
            _ => {
                permission_requests().remove(&request_id);
                // Notify frontend that this permission request timed out / was cancelled
                if let Ok(guard) = self.delta_slot.lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.try_send(AcpEvent::PermissionExpired {
                            request_id: request_id.clone(),
                        });
                    }
                }
                Ok(acp::RequestPermissionResponse::new(
                    acp::RequestPermissionOutcome::Cancelled,
                ))
            }
        }
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        let event = match args.update {
            acp::SessionUpdate::UserMessageChunk(chunk) => {
                let text = match chunk.content {
                    acp::ContentBlock::Text(tc) => tc.text,
                    acp::ContentBlock::ResourceLink(rl) => rl.uri,
                    _ => return Ok(()),
                };
                if text.is_empty() {
                    return Ok(());
                }
                AcpEvent::UserTextDelta { text }
            }
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                let text = match chunk.content {
                    acp::ContentBlock::Text(tc) => tc.text,
                    acp::ContentBlock::ResourceLink(rl) => rl.uri,
                    _ => return Ok(()),
                };
                if text.is_empty() {
                    return Ok(());
                }
                AcpEvent::TextDelta { text }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                let text = match chunk.content {
                    acp::ContentBlock::Text(tc) => tc.text,
                    _ => return Ok(()),
                };
                if text.is_empty() {
                    return Ok(());
                }
                AcpEvent::ThoughtDelta { text }
            }
            acp::SessionUpdate::ToolCall(tc) => AcpEvent::ToolCall {
                tool_call_id: tc.tool_call_id.to_string(),
                title: tc.title.clone(),
                tool_kind: serde_json::to_value(&tc.kind)
                    .and_then(|v| Ok(v.as_str().unwrap_or("unknown").to_string()))
                    .unwrap_or_else(|_| "unknown".to_string()),
                status: serde_json::to_value(&tc.status)
                    .and_then(|v| Ok(v.as_str().unwrap_or("pending").to_string()))
                    .unwrap_or_else(|_| "pending".to_string()),
                content: if tc.content.is_empty() {
                    None
                } else {
                    Some(
                        tc.content
                            .iter()
                            .map(|c| serde_json::to_value(c).unwrap_or(json!({})))
                            .collect(),
                    )
                },
                locations: if tc.locations.is_empty() {
                    None
                } else {
                    Some(
                        tc.locations
                            .iter()
                            .map(|loc| serde_json::to_value(loc).unwrap_or(json!({})))
                            .collect(),
                    )
                },
                raw_input: cap_raw(tc.raw_input.clone()),
                raw_output: cap_raw(tc.raw_output.clone()),
            },
            acp::SessionUpdate::ToolCallUpdate(tcu) => AcpEvent::ToolCallUpdate {
                tool_call_id: tcu.tool_call_id.to_string(),
                tool_kind: tcu.fields.kind.map(|k| {
                    serde_json::to_value(&k)
                        .and_then(|v| Ok(v.as_str().unwrap_or("").to_string()))
                        .unwrap_or_default()
                }),
                status: tcu.fields.status.map(|s| {
                    serde_json::to_value(&s)
                        .and_then(|v| Ok(v.as_str().unwrap_or("").to_string()))
                        .unwrap_or_default()
                }),
                title: tcu.fields.title.clone(),
                content: tcu.fields.content.as_ref().map(|blocks| {
                    blocks
                        .iter()
                        .map(|b| serde_json::to_value(b).unwrap_or(json!({})))
                        .collect()
                }),
                locations: tcu.fields.locations.as_ref().map(|items| {
                    items
                        .iter()
                        .map(|loc| serde_json::to_value(loc).unwrap_or(json!({})))
                        .collect()
                }),
                raw_input: cap_raw(tcu.fields.raw_input.clone()),
                raw_output: cap_raw(tcu.fields.raw_output.clone()),
            },
            acp::SessionUpdate::Plan(plan) => AcpEvent::Plan {
                entries: plan
                    .entries
                    .iter()
                    .map(|e| serde_json::to_value(e).unwrap_or(json!({})))
                    .collect(),
            },
            acp::SessionUpdate::CurrentModeUpdate(mode) => AcpEvent::ModeUpdate {
                mode_id: mode.current_mode_id.to_string(),
            },
            acp::SessionUpdate::ConfigOptionUpdate(cfg) => AcpEvent::ConfigUpdate {
                options: cfg
                    .config_options
                    .iter()
                    .map(|o| serde_json::to_value(o).unwrap_or(json!({})))
                    .collect(),
            },
            acp::SessionUpdate::SessionInfoUpdate(info) => AcpEvent::SessionInfo {
                title: match info.title {
                    acp::MaybeUndefined::Value(v) => Some(v),
                    _ => None,
                },
            },
            acp::SessionUpdate::AvailableCommandsUpdate(cmds) => AcpEvent::AvailableCommands {
                commands: cmds
                    .available_commands
                    .iter()
                    .map(|c| serde_json::to_value(c).unwrap_or(json!({})))
                    .collect(),
            },
            _ => return Ok(()),
        };
        if let Ok(guard) = self.delta_slot.lock() {
            if let Some(tx) = guard.as_ref() {
                if tx.try_send(event).is_err() {
                    // Channel full or disconnected — log once per overflow to aid debugging
                    use super::adapter::acp_log;
                    acp_log(
                        "delta.channel.drop",
                        serde_json::json!({ "capacity": super::worker::DELTA_CHANNEL_CAPACITY }),
                    );
                }
            }
        }
        Ok(())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let path = args.path;
        let full = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;

        // Apply optional line/limit slicing (1-based per spec)
        let content = match (args.line, args.limit) {
            (None, None) => full,
            (start, limit) => {
                let start_idx = start.map(|l| l.saturating_sub(1) as usize).unwrap_or(0);
                let lines: Vec<&str> = full.lines().collect();
                let slice = &lines[start_idx.min(lines.len())..];
                let slice = if let Some(n) = limit {
                    &slice[..n.min(slice.len() as u32) as usize]
                } else {
                    slice
                };
                slice.join("\n")
            }
        };

        Ok(acp::ReadTextFileResponse::new(content))
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        if let Some(parent) = args.path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&args.path, &args.content)
            .await
            .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;
        Ok(acp::WriteTextFileResponse::new())
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        super::client_terminal::create_terminal(args).await
    }
    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        super::client_terminal::terminal_output(args).await
    }
    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        super::client_terminal::wait_for_terminal_exit(args).await
    }

    async fn kill_terminal(
        &self,
        args: acp::KillTerminalRequest,
    ) -> acp::Result<acp::KillTerminalResponse> {
        super::client_terminal::kill_terminal(args).await
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        super::client_terminal::release_terminal(args).await
    }
}
