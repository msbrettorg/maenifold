use agent_client_protocol::{self as acp};
use serde_json::json;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, build_stdio_adapter, friendly_error_message};
use super::super::worker::{AcpEvent, AcpPromptResult, WorkerMsg, worker_tx};
use super::cold_start::default_client_capabilities;
use super::execute_stream::{error_code_from_raw, mock_execute, run_delta_loop};
// use crate::db::app_session_role::{load_app_session_role_cli_id, save_app_session_role_cli_id};
// TODO: wire session ID persistence through Jockey DB
// use crate::types::AppState;

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
    _state: Option<(&crate::types::AppState, &str)>,
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
            };
        }
        Err(e) => {
            return AcpPromptResult {
                ok: false,
                output: friendly_error_message(&normalized, &e),
                error_code: Some("ADAPTER_UNAVAILABLE".to_string()),
                deltas: vec![],
                meta: json!({ "mode": "adapter-unavailable", "runtime": normalized, "error": e }),
            };
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

    let (delta_tx, delta_rx) =
        mpsc::channel::<AcpEvent>(super::super::worker::DELTA_CHANNEL_CAPACITY);
    let (result_tx, result_rx) = oneshot::channel();

    let app_session_id_owned = app_session_scope.clone();

    let _ = worker_tx().send(WorkerMsg::Execute {
        runtime_key: runtime_key.clone(),
        binary: adapter.binary.clone(),
        args: adapter.args.clone(),
        env: adapter.env.clone(),
        client_capabilities: default_client_capabilities(),
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

    let (result, full_output, delta_count) = run_delta_loop(
        delta_rx,
        result_rx,
        event_tx,
        &runtime_key,
        role_name,
        &app_session_id_owned,
        prompt,
    )
    .await;

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
            let friendly = friendly_error_message(&runtime_key, &e);
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
