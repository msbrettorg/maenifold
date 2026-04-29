use agent_client_protocol::{self as acp};
use serde_json::json;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, build_stdio_adapter, friendly_error_message};
use super::super::worker::{AcpEvent, AcpExecuteError, AcpPromptResult, WorkerMsg, worker_tx};
use super::cold_start::default_client_capabilities;
use super::execute_stream::{error_code_from_raw, run_delta_loop};
use super::resume::sanitized_resume_failure_kind;

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
    resume_session_id: Option<String>,
) -> AcpPromptResult {
    let normalized = runtime_kind.trim().to_ascii_lowercase();
    let requested_resume = resume_session_id.is_some();

    if normalized.is_empty() {
        let output =
            "ACP runtime_kind is required; mock or empty runtimes are not allowed in production"
                .to_string();
        return AcpPromptResult {
            ok: false,
            output: output.clone(),
            error_code: Some("INVALID_RUNTIME".to_string()),
            deltas: vec![],
            meta: json!({
                "mode": "invalid-runtime",
                "runtime": runtime_kind,
                "requestedResume": requested_resume,
                "resumed": false,
                "resumeFailureKind": sanitized_resume_failure_kind(&output)
            }),
        };
    }

    if normalized == "mock" {
        let output =
            "ACP runtime_kind 'mock' is not allowed in production; configure a real ACP runtime"
                .to_string();
        return AcpPromptResult {
            ok: false,
            output: output.clone(),
            error_code: Some("INVALID_RUNTIME".to_string()),
            deltas: vec![],
            meta: json!({
                "mode": "invalid-runtime",
                "runtime": normalized,
                "requestedResume": requested_resume,
                "resumed": false,
                "resumeFailureKind": sanitized_resume_failure_kind(&output)
            }),
        };
    }

    let adapter = match build_stdio_adapter(&normalized) {
        Ok(Some(a)) => a,
        Ok(None) => {
            let output = format!("unsupported runtime kind: {}", normalized);
            return AcpPromptResult {
                ok: false,
                output: output.clone(),
                error_code: Some("UNSUPPORTED_RUNTIME".to_string()),
                deltas: vec![],
                meta: json!({
                    "mode": "unsupported-runtime",
                    "runtime": normalized,
                    "requestedResume": requested_resume,
                    "resumed": false,
                    "resumeFailureKind": sanitized_resume_failure_kind(&output)
                }),
            };
        }
        Err(e) => {
            let friendly = friendly_error_message(&normalized, &e);
            return AcpPromptResult {
                ok: false,
                output: friendly.clone(),
                error_code: Some("ADAPTER_UNAVAILABLE".to_string()),
                deltas: vec![],
                meta: json!({
                    "mode": "adapter-unavailable",
                    "runtime": normalized,
                    "error": e,
                    "requestedResume": requested_resume,
                    "resumed": false,
                    "resumeFailureKind": sanitized_resume_failure_kind(&friendly)
                }),
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

    let resume_session_id = resume_session_id;
    let app_session_scope = if app_session_id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        app_session_id.to_string()
    };

    let (delta_tx, delta_rx) =
        mpsc::channel::<AcpEvent>(super::super::worker::DELTA_CHANNEL_CAPACITY);
    let (result_tx, result_rx) = oneshot::channel();

    let app_session_id_owned = app_session_scope.clone();

    if worker_tx()
        .send(WorkerMsg::Execute {
            runtime_key: runtime_key.clone(),
            binary: adapter.binary.clone(),
            args: adapter.args.clone(),
            env: adapter.env.clone(),
            client_capabilities: default_client_capabilities(),
            role_name: role_name.to_string(),
            app_session_id: app_session_scope,
            prompt: prompt.to_string(),
            context: context.to_vec(),
            prompt_blocks: None,
            cwd: cwd.to_string(),
            delta_tx,
            result_tx,
            auto_approve,
            mcp_servers,
            role_mode,
            role_config_options,
            resume_session_id,
        })
        .is_err()
    {
        let message = "ACP worker unavailable".to_string();
        return AcpPromptResult {
            ok: false,
            output: message.clone(),
            error_code: Some("ADAPTER_UNAVAILABLE".to_string()),
            deltas: vec![],
            meta: json!({
                "mode": "acp-error",
                "runtime": runtime_key,
                "error": message,
                "friendlyMessage": "ACP worker unavailable",
                "requestedResume": requested_resume,
                "resumed": false,
                "resumeFailureKind": requested_resume.then(|| "startup")
            }),
        };
    }

    let (result, full_output, delta_count) = run_delta_loop(
        delta_rx,
        result_rx,
        event_tx,
        &runtime_key,
        role_name,
        &app_session_id_owned,
        prompt,
        requested_resume,
    )
    .await;

    match result {
        Ok(success) => {
            let resumed = success.resumed;
            let session_id = success.session_id;
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
                if success.output.is_empty() {
                    full_output
                } else {
                    success.output
                }
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
                    "sessionId": session_id,
                    "resumed": resumed,
                    "requestedResume": requested_resume
                }),
            }
        }
        Err(e) => {
            let friendly = friendly_error_message(&runtime_key, &e.message);
            let code = error_code_from_raw(&e.message);
            let AcpExecuteError {
                message,
                requested_resume,
                resumed,
                resume_failure_kind,
            } = e;
            let meta = match resume_failure_kind {
                Some(kind) => json!({
                    "mode": "acp-error",
                    "runtime": runtime_key,
                    "error": message,
                    "friendlyMessage": friendly,
                    "requestedResume": requested_resume,
                    "resumed": resumed,
                    "resumeFailureKind": kind
                }),
                None => json!({
                    "mode": "acp-error",
                    "runtime": runtime_key,
                    "error": message,
                    "friendlyMessage": friendly,
                    "requestedResume": requested_resume,
                    "resumed": resumed
                }),
            };
            AcpPromptResult {
                ok: false,
                output: friendly.clone(),
                error_code: Some(code),
                deltas: vec![],
                meta,
            }
        }
    }
}
