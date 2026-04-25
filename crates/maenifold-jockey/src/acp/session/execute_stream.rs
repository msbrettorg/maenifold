use crate::emitter::Emitter;
use serde::Serialize;
use serde_json::json;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, clip};
use super::super::runtime_state::{
    remember_runtime_available_commands, remember_runtime_config_options,
};
use super::super::worker::AcpEvent;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AcpDeltaPayload<'a> {
    pub role: &'a str,
    pub runtime_kind: &'a str,
    pub app_session_id: &'a str,
    pub delta: &'a str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AcpStreamPayload<'a> {
    pub role: &'a str,
    pub runtime_kind: &'a str,
    pub app_session_id: &'a str,
    pub event: &'a AcpEvent,
}

pub(super) async fn run_delta_loop(
    mut delta_rx: mpsc::Receiver<AcpEvent>,
    mut result_rx: oneshot::Receiver<Result<(String, String), String>>,
    event_tx: &crate::EventSender,
    runtime_key: &str,
    role_name: &str,
    app_session_id: &str,
    prompt: &str,
) -> (Result<(String, String), String>, String, usize) {
    let mut full_output = String::new();
    let mut delta_count = 0usize;
    let mut delta_batch: String = String::new();

    acp_log(
        "execute.stream.listening",
        json!({
            "runtime": runtime_key,
            "role": role_name,
            "prompt": clip(prompt, 80),
        }),
    );

    let heartbeat = tokio::time::Instant::now();
    let mut heartbeat_count = 0u32;
    let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(5));
    heartbeat_interval.tick().await; // consume the immediate first tick
    let mut flush_interval = tokio::time::interval(std::time::Duration::from_millis(30));
    flush_interval.tick().await; // consume the immediate first tick

    let result = loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                heartbeat_count += 1;
                acp_log("execute.heartbeat", json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "elapsedSec": heartbeat.elapsed().as_secs(),
                    "deltaCount": delta_count,
                    "beat": heartbeat_count,
                }));
                continue;
            }
            _ = flush_interval.tick() => {
                if !delta_batch.is_empty() {
                    let _ = event_tx.emit(
                        "acp/delta",
                        AcpDeltaPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            delta: &delta_batch,
                        },
                    );
                    delta_batch.clear();
                }
                continue;
            }
            evt = delta_rx.recv() => {
                match evt {
                    Some(AcpEvent::TextDelta { ref text }) => {
                        full_output.push_str(text);
                        delta_count += 1;
                        acp_log("delta.text", json!({
                            "runtime": runtime_key,
                            "role": role_name,
                            "deltaIndex": delta_count,
                            "chunkLen": text.len(),
                            "preview": clip(text, 60),
                        }));
                        delta_batch.push_str(text);
                    }
                    Some(ref evt @ AcpEvent::ConfigUpdate { ref options }) => {
                        delta_count += 1;
                        remember_runtime_config_options(
                            runtime_key,
                            options.clone(),
                        );
                        let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            event: evt,
                        });
                    }
                    Some(ref evt @ AcpEvent::AvailableCommands { ref commands }) => {
                        delta_count += 1;
                        acp_log("commands.discovered", json!({
                            "runtime": runtime_key,
                            "role": role_name,
                            "count": commands.len(),
                            "names": commands.iter().filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())).collect::<Vec<_>>()
                        }));
                        remember_runtime_available_commands(
                            app_session_id,
                            runtime_key,
                            role_name,
                            commands.clone(),
                        );
                        let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            event: evt,
                        });
                    }
                    Some(ref other) => {
                        delta_count += 1;
                        acp_log("delta.event", json!({
                            "runtime": runtime_key,
                            "role": role_name,
                            "deltaIndex": delta_count,
                        }));
                        let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            event: other,
                        });
                    }
                    None => {}
                }
            }
            res = &mut result_rx => {
                if !delta_batch.is_empty() {
                    let _ = event_tx.emit(
                        "acp/delta",
                        AcpDeltaPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            delta: &delta_batch,
                        },
                    );
                    delta_batch.clear();
                }
                while let Ok(evt) = delta_rx.try_recv() {
                    match evt {
                        AcpEvent::TextDelta { ref text } => {
                            full_output.push_str(text);
                            delta_count += 1;
                            delta_batch.push_str(text);
                        }
                        ref other => {
                            delta_count += 1;
                            let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                                role: role_name,
                                runtime_kind: runtime_key,
                                app_session_id,
                                event: other,
                            });
                        }
                    }
                }
                if !delta_batch.is_empty() {
                    let _ = event_tx.emit(
                        "acp/delta",
                        AcpDeltaPayload {
                            role: role_name,
                            runtime_kind: runtime_key,
                            app_session_id,
                            delta: &delta_batch,
                        },
                    );
                    delta_batch.clear();
                }
                let r = res.unwrap_or_else(|_| Err("worker disconnected".to_string()));
                acp_log("execute.result", json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "ok": r.is_ok(),
                    "deltaCount": delta_count,
                    "outputSize": full_output.len(),
                    "outputPreview": clip(&full_output, 120),
                }));
                break r;
            }
        }
    };

    (result, full_output, delta_count)
}

pub(super) fn error_code_from_raw(raw: &str) -> String {
    let l = raw.to_ascii_lowercase();
    if l.contains("429")
        || l.contains("rate limit")
        || l.contains("quota")
        || l.contains("too many requests")
    {
        "RATE_LIMITED".to_string()
    } else if l.contains("epipe")
        || l.contains("broken pipe")
        || l.contains("transport closed")
        || (l.contains("agent '") && l.contains(" exited:"))
        || l.contains("agent process exited")
        || l.contains("no longer alive")
    {
        "PROCESS_CRASHED".to_string()
    } else if l.contains("timeout") {
        "TIMEOUT".to_string()
    } else if l.contains("binary not found")
        || l.contains("adapter unavailable")
        || l.contains("not found in maenifold.json")
        || l.contains("failed to start agent")
    {
        "ADAPTER_UNAVAILABLE".to_string()
    } else if l.contains("missing --experimental-acp") {
        "INCOMPATIBLE_VERSION".to_string()
    } else {
        "ACP_ERROR".to_string()
    }
}
