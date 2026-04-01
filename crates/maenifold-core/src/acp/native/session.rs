/// ACP session management — new session, prompt, timeout, and cleanup.
use acp::Agent;
use agent_client_protocol as acp;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    sync::Mutex,
    time::{Duration, timeout},
};

use super::accumulator::SessionAccumulator;
use super::types::{AcpxUsage, AgentError, AgentResponse, Result, SessionInfo, SessionOpts};

pub(super) async fn handle_new_session(
    conn: &acp::ClientSideConnection,
    opts: SessionOpts,
) -> Result<SessionInfo> {
    let response = conn
        .new_session(
            acp::NewSessionRequest::new(opts.cwd.clone()).mcp_servers(opts.mcp_servers.clone()),
        )
        .await
        .map_err(|error| AgentError::Acp(error.to_string()))?;
    let sid = response.session_id.clone();
    let session_info = SessionInfo {
        session_id: sid.to_string(),
        modes: response
            .modes
            .as_ref()
            .map(|state| state.available_modes.clone())
            .unwrap_or_default(),
        models: response
            .models
            .as_ref()
            .map(|state| state.available_models.clone())
            .unwrap_or_default(),
        config_options: response.config_options.clone().unwrap_or_default(),
    };
    apply_session_opts(conn, sid, &opts).await;
    Ok(session_info)
}

async fn apply_session_opts(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
) {
    apply_session_config(
        conn,
        session_id,
        opts.model.clone(),
        opts.mode.clone(),
        opts.thought_level.clone(),
        opts.config_options.clone(),
        opts.allowed_tools.clone(),
        opts.max_turns,
    )
    .await;
}

pub(super) async fn apply_session_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    model: Option<String>,
    mode: Option<String>,
    thought_level: Option<String>,
    config_options: Option<serde_json::Map<String, serde_json::Value>>,
    allowed_tools: Option<Vec<String>>,
    max_turns: Option<u32>,
) {
    apply_optional(model, "ACP session model", |model| {
        conn.set_session_model(acp::SetSessionModelRequest::new(session_id.clone(), model))
    })
    .await;
    apply_optional(mode, "ACP session mode", |mode| {
        conn.set_session_mode(acp::SetSessionModeRequest::new(session_id.clone(), mode))
    })
    .await;
    set_config_options(conn, session_id.clone(), config_options).await;
    set_config_option(conn, session_id.clone(), "thought_level", thought_level).await;
    set_config_option(
        conn,
        session_id.clone(),
        "allowed_tools",
        allowed_tools
            .map(|tools| serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())),
    )
    .await;
    set_config_option(
        conn,
        session_id,
        "max_turns",
        max_turns.map(|turns| turns.to_string()),
    )
    .await;
}

async fn set_config_options(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    config_options: Option<serde_json::Map<String, serde_json::Value>>,
) {
    if let Some(config_options) = config_options {
        for (config_id, value) in config_options {
            if is_built_in_config_option(&config_id) {
                continue;
            }
            set_config_option(
                conn,
                session_id.clone(),
                &config_id,
                Some(config_option_value(value)),
            )
            .await;
        }
    }
}

fn is_built_in_config_option(config_id: &str) -> bool {
    matches!(config_id, "thought_level" | "allowed_tools" | "max_turns")
}

fn config_option_value(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value,
        value => value.to_string(),
    }
}

async fn apply_optional<T, F, Fut, R>(value: Option<T>, description: &str, apply: F)
where
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = acp::Result<R>>,
{
    if let Some(value) = value
        && let Err(error) = apply(value).await
    {
        tracing::warn!("Failed to set {description}: {error}");
    }
}

async fn set_config_option(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    config_id: &str,
    value: Option<String>,
) {
    apply_optional(value, &format!("ACP session config {config_id}"), |value| {
        conn.set_session_config_option(acp::SetSessionConfigOptionRequest::new(
            session_id,
            config_id.to_string(),
            value,
        ))
    })
    .await;
}

async fn with_session_accumulator<T, F>(
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: &str,
    future: F,
) -> (T, SessionAccumulator)
where
    F: std::future::Future<Output = T>,
{
    sessions
        .lock()
        .await
        .insert(session_id.to_string(), SessionAccumulator::default());
    let output = future.await;
    let accumulator = sessions.lock().await.remove(session_id).unwrap_or_default();
    (output, accumulator)
}

fn format_stop_reason(stop_reason: acp::StopReason) -> String {
    serde_json::to_value(stop_reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) async fn handle_prompt(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: &str,
    prompt: &str,
) -> Result<AgentResponse> {
    let (response, accumulator) = with_session_accumulator(sessions, session_id, async {
        conn.prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.to_string()),
            vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))],
        ))
        .await
        .map_err(|error| AgentError::Acp(error.to_string()))
    })
    .await;
    let mut result = accumulator.into_response();
    match response {
        Ok(response) => {
            result.stop_reason = Some(format_stop_reason(response.stop_reason));
            result.usage = response.usage.map(|u| AcpxUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
                thought_tokens: u.thought_tokens,
            });
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

pub(super) async fn cleanup_timed_out_prompt(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: &str,
) {
    sessions.lock().await.remove(session_id);
    let session_id = acp::SessionId::new(session_id.to_string());
    if let Err(error) = conn
        .cancel(acp::CancelNotification::new(session_id.clone()))
        .await
    {
        tracing::warn!("Failed to cancel timed out ACP prompt: {error}");
    }
    if let Err(error) = conn
        .close_session(acp::CloseSessionRequest::new(session_id))
        .await
    {
        tracing::warn!("Failed to close timed out ACP session: {error}");
    }
}

pub(super) async fn run_with_timeout<T, O, C>(
    duration: Duration,
    operation: O,
    on_timeout: C,
    timeout_message: String,
) -> Result<T>
where
    O: std::future::Future<Output = Result<T>>,
    C: std::future::Future<Output = ()>,
{
    match timeout(duration, operation).await {
        Ok(result) => result,
        Err(_) => {
            on_timeout.await;
            Err(AgentError::Acp(timeout_message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentError, run_with_timeout};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn run_with_timeout_returns_operation_result_before_deadline() {
        let cleanups = Arc::new(AtomicUsize::new(0));
        let result = run_with_timeout(
            Duration::from_millis(50),
            async { Ok::<_, AgentError>("ok") },
            {
                let cleanups = Arc::clone(&cleanups);
                async move {
                    cleanups.fetch_add(1, Ordering::SeqCst);
                }
            },
            "prompt timed out".to_string(),
        )
        .await
        .expect("operation should complete");

        assert_eq!(result, "ok");
        assert_eq!(cleanups.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn run_with_timeout_runs_cleanup_when_operation_times_out() {
        let cleanups = Arc::new(AtomicUsize::new(0));
        let result = run_with_timeout(
            Duration::from_millis(10),
            async {
                sleep(Duration::from_millis(50)).await;
                Ok::<_, AgentError>("late")
            },
            {
                let cleanups = Arc::clone(&cleanups);
                async move {
                    cleanups.fetch_add(1, Ordering::SeqCst);
                }
            },
            "prompt timed out".to_string(),
        )
        .await;

        assert!(matches!(result, Err(AgentError::Acp(message)) if message == "prompt timed out"));
        assert_eq!(cleanups.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn config_option_value_preserves_string_values() {
        assert_eq!(
            super::config_option_value(serde_json::Value::String("4096".to_string())),
            "4096"
        );
    }

    #[test]
    fn config_option_value_stringifies_non_string_values() {
        assert_eq!(super::config_option_value(serde_json::json!(true)), "true");
        assert_eq!(super::config_option_value(serde_json::json!(42)), "42");
    }

    #[test]
    fn built_in_config_options_are_filtered_from_custom_entries() {
        assert!(super::is_built_in_config_option("thought_level"));
        assert!(super::is_built_in_config_option("allowed_tools"));
        assert!(super::is_built_in_config_option("max_turns"));
        assert!(!super::is_built_in_config_option("sandbox"));
    }
}
