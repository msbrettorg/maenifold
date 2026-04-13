use agent_client_protocol::{self as acp};
use serde::Serialize;
use serde_json::json;
use std::time::Instant;
use crate::emitter::Emitter;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, build_stdio_adapter, clip, friendly_error_message};
use super::super::runtime_state::{
    remember_runtime_available_commands, remember_runtime_config_options,
};
use super::super::worker::{worker_tx, AcpEvent, AcpPromptResult, WorkerMsg};
// use crate::db::app_session_role::{load_app_session_role_cli_id, save_app_session_role_cli_id};
// TODO: wire session ID persistence through Jockey DB
// use crate::types::AppState;

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

pub async fn execute_runtime(
    runtime_kind: &str,
    role_name: &str,
    prompt: &str,
    context: &[(String, String)],
    cwd: &str,
    event_tx: &crate::EventSender,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    role_mode: Option<String>,
    role_config_options: Vec<(String, String)>,
    state: Option<(&crate::types::AppState, &str)>,
    app_session_id: &str,
) -> AcpPromptResult {
    let normalized = runtime_kind.trim().to_ascii_lowercase();

    if normalized.is_empty() || normalized == "mock" {
        return mock_execute(role_name, prompt, context);
    }

    let adapter = match build_stdio_adapter(&normalized) {
        Ok(Some(a)) => a,
        Ok(None) => {
            return AcpPromptResult {
                ok: false,
                output: format!("unsupported runtime kind: {}", normalized),
                error_code: Some("UNSUPPORTED_RUNTIME".to_string()),
                deltas: vec![],
                meta: json!({ "mode": "unsupported-runtime", "runtime": normalized }),
            }
        }
        Err(e) => {
            return AcpPromptResult {
                ok: false,
                output: friendly_error_message(&normalized, &e),
                error_code: Some("ADAPTER_UNAVAILABLE".to_string()),
                deltas: vec![],
                meta: json!({ "mode": "adapter-unavailable", "runtime": normalized, "error": e }),
            }
        }
    };

    let runtime_key = adapter.runtime_key.clone();
    let started = Instant::now();
    acp_log(
        "execute.start",
        json!({
            "runtime": runtime_key,
            "role": role_name,
            "promptSize": prompt.len(),
            "cwd": cwd
        }),
    );

    // TODO: wire session ID persistence through Jockey DB
    // let resume_session_id = state.as_ref().and_then(|(s, app_sid)| {
    //     load_app_session_role_cli_id(s, app_sid, runtime_key, role_name)
    // });
    let resume_session_id = None;
    let app_session_scope = if app_session_id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        app_session_id.to_string()
    };

    let (delta_tx, mut delta_rx) =
        mpsc::channel::<AcpEvent>(super::super::worker::DELTA_CHANNEL_CAPACITY);
    let (result_tx, mut result_rx) = oneshot::channel();

    let app_session_id_owned = app_session_scope.clone();

    let _ = worker_tx().send(WorkerMsg::Execute {
        runtime_key: runtime_key.clone(),
        binary: adapter.binary.clone(),
        args: adapter.args.clone(),
        env: adapter.env.clone(),
        role_name: role_name.to_string(),
        app_session_id: app_session_scope,
        prompt: prompt.to_string(),
        context: context.to_vec(),
        cwd: cwd.to_string(),
        delta_tx,
        result_tx,
        auto_approve,
        mcp_servers,
        role_mode,
        role_config_options,
        resume_session_id,
    });

    let event_tx = event_tx.clone();
    let role_owned = role_name.to_string();
    let mut full_output = String::new();
    let mut delta_count = 0usize;
    // Buffer for batching TextDelta IPC events (~30ms flush interval)
    let mut delta_batch: String = String::new();

    acp_log(
        "execute.stream.listening",
        json!({
            "runtime": runtime_key,
            "role": role_owned,
            "prompt": clip(prompt, 80),
        }),
    );

    // Flush helper: emit buffered text deltas as a single IPC event
    macro_rules! flush_delta_batch {
        () => {
            if !delta_batch.is_empty() {
                let _ = event_tx.emit(
                    "acp/delta",
                    AcpDeltaPayload {
                        role: &role_owned,
                        runtime_kind: &runtime_key,
                        app_session_id: &app_session_id_owned,
                        delta: &delta_batch,
                    },
                );
                delta_batch.clear();
            }
        };
    }

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
                    "role": role_owned,
                    "elapsedSec": heartbeat.elapsed().as_secs(),
                    "deltaCount": delta_count,
                    "beat": heartbeat_count,
                }));
                continue;
            }
            _ = flush_interval.tick() => {
                if !delta_batch.is_empty() {
                    flush_delta_batch!();
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
                            "role": role_owned,
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
                            role: &role_owned,
                            runtime_kind: &runtime_key,
                            app_session_id: &app_session_id_owned,
                            event: evt,
                        });
                    }
                    Some(ref evt @ AcpEvent::AvailableCommands { ref commands }) => {
                        delta_count += 1;
                        acp_log("commands.discovered", json!({
                            "runtime": runtime_key,
                            "role": role_owned,
                            "count": commands.len(),
                            "names": commands.iter().filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())).collect::<Vec<_>>()
                        }));
                        remember_runtime_available_commands(
                            &app_session_id_owned,
                            runtime_key,
                            &role_owned,
                            commands.clone(),
                        );
                        let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                            role: &role_owned,
                            runtime_kind: &runtime_key,
                            app_session_id: &app_session_id_owned,
                            event: evt,
                        });
                    }
                    Some(ref other) => {
                        delta_count += 1;
                        acp_log("delta.event", json!({
                            "runtime": runtime_key,
                            "role": role_owned,
                            "deltaIndex": delta_count,
                        }));
                        let _ = event_tx.emit("acp/stream", AcpStreamPayload {
                            role: &role_owned,
                            runtime_kind: &runtime_key,
                            app_session_id: &app_session_id_owned,
                            event: other,
                        });
                    }
                    None => {}
                }
            }
            res = &mut result_rx => {
                // Flush any buffered deltas before draining the channel
                flush_delta_batch!();
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
                                role: &role_owned,
                                runtime_kind: &runtime_key,
                                app_session_id: &app_session_id_owned,
                                event: other,
                            });
                        }
                    }
                }
                // Emit any remaining buffered text from drain
                flush_delta_batch!();
                let r = res.unwrap_or_else(|_| Err("worker disconnected".to_string()));
                acp_log("execute.result", json!({
                    "runtime": runtime_key,
                    "role": role_owned,
                    "ok": r.is_ok(),
                    "deltaCount": delta_count,
                    "outputSize": full_output.len(),
                    "outputPreview": clip(&full_output, 120),
                }));
                break r;
            }
        }
    };

    match result {
        Ok((_output, session_id)) => {
            // TODO: wire session ID persistence through Jockey DB
            // if let Some((s, app_sid)) = state {
            //     if !session_id.is_empty() {
            //         let _ = save_app_session_role_cli_id(
            //             s,
            //             app_sid,
            //             runtime_key,
            //             role_name,
            //             &session_id,
            //         );
            //     }
            // }

            let output = if full_output.is_empty() {
                if prompt.starts_with('/') {
                    format!(
                        "Command sent to {} (role: {}). Agent processed it with {} event(s) but returned no text output.",
                        runtime_key, role_name, delta_count
                    )
                } else {
                    // Non-command prompts that produce no text output (e.g. cancelled
                    // or tool-only turns) should be silently ignored by the frontend.
                    String::new()
                }
            } else {
                full_output
            };

            acp_log(
                "execute.ok",
                json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "latencyMs": started.elapsed().as_millis(),
                    "deltaCount": delta_count,
                    "outputSize": output.len()
                }),
            );

            AcpPromptResult {
                ok: true,
                output,
                error_code: None,
                deltas: vec![],
                meta: json!({
                    "mode": "live",
                    "agentKind": runtime_key,
                    "runtimeKey": runtime_key,
                    "sessionId": session_id
                }),
            }
        }
        Err(e) => {
            let friendly = friendly_error_message(runtime_key, &e);
            let code = error_code_from_raw(&e);
            AcpPromptResult {
                ok: false,
                output: friendly.clone(),
                error_code: Some(code),
                deltas: vec![],
                meta: json!({ "mode": "acp-error", "runtime": runtime_key, "error": e, "friendlyMessage": friendly }),
            }
        }
    }
}

pub(super) fn error_code_from_raw(raw: &str) -> String {
    let l = raw.to_ascii_lowercase();
    if l.contains("429")
        || l.contains("rate limit")
        || l.contains("quota")
        || l.contains("too many requests")
    {
        "RATE_LIMITED".to_string()
    } else if l.contains("epipe") || l.contains("broken pipe") || l.contains("transport closed") {
        "PROCESS_CRASHED".to_string()
    } else if l.contains("timeout") {
        "TIMEOUT".to_string()
    } else if l.contains("binary not found") || l.contains("adapter unavailable") {
        "ADAPTER_UNAVAILABLE".to_string()
    } else if l.contains("missing --experimental-acp") {
        "INCOMPATIBLE_VERSION".to_string()
    } else {
        "ACP_ERROR".to_string()
    }
}

pub(super) fn mock_execute(role: &str, prompt: &str, ctx: &[(String, String)]) -> AcpPromptResult {
    let snap = ctx
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");
    let out = format!("{role} prompt: {prompt}. Context: {snap}.");
    AcpPromptResult {
        ok: true,
        output: out.clone(),
        error_code: None,
        deltas: out
            .as_bytes()
            .chunks(28)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect(),
        meta: json!({ "mode": "mock", "agentKind": "mock", "runtimeKey": "mock" }),
    }
}
