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
    let SessionOpts {
        cwd,
        model,
        mode,
        thought_level,
        config_options,
        allowed_tools,
        max_turns,
        timeout_seconds: _,
        mcp_servers,
    } = opts;
    let response = conn
        .new_session(acp::NewSessionRequest::new(cwd).mcp_servers(mcp_servers))
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
    if let Some(model) = model
        && let Err(error) = conn
            .set_session_model(acp::SetSessionModelRequest::new(sid.clone(), model))
            .await
    {
        tracing::warn!("Failed to set ACP session model: {error}");
    }
    set_mode(conn, sid.clone(), mode).await;
    set_config_options(conn, sid.clone(), config_options).await;
    set_option(conn, sid.clone(), "thought_level", thought_level).await;
    set_option(
        conn,
        sid.clone(),
        "allowed_tools",
        allowed_tools
            .map(|tools| serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())),
    )
    .await;
    set_option(
        conn,
        sid.clone(),
        "max_turns",
        max_turns.map(|turns| turns.to_string()),
    )
    .await;
    Ok(session_info)
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
            set_option(
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

async fn set_option(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    config_id: &str,
    value: Option<String>,
) {
    if let Some(value) = value
        && let Err(error) = conn
            .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                session_id,
                config_id.to_string(),
                value,
            ))
            .await
    {
        tracing::warn!("Failed to set ACP session config {config_id}: {error}");
    }
}

async fn set_mode(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    mode: Option<String>,
) {
    if let Some(mode) = mode
        && let Err(error) = conn
            .set_session_mode(acp::SetSessionModeRequest::new(session_id, mode))
            .await
    {
        tracing::warn!("Failed to set ACP session mode: {error}");
    }
}

pub(super) async fn handle_prompt(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: &str,
    prompt: &str,
) -> Result<AgentResponse> {
    sessions
        .lock()
        .await
        .insert(session_id.to_string(), SessionAccumulator::default());
    let response = conn
        .prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.to_string()),
            vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))],
        ))
        .await
        .map_err(|error| AgentError::Acp(error.to_string()));
    let mut result = sessions
        .lock()
        .await
        .remove(session_id)
        .unwrap_or_default()
        .into_response();
    let response = response?;
    result.stop_reason = Some(format!("{:?}", response.stop_reason));
    result.usage = response.usage.map(|u| AcpxUsage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        total_tokens: u.total_tokens,
        thought_tokens: u.thought_tokens,
    });
    Ok(result)
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
