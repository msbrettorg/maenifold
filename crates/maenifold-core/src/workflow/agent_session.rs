use jockey::acp::AcpPromptResult;
use serde_json::{Map, Value};
use tracing::{info, warn};

use crate::acp_config::AgentsConfig;
use crate::error::{MaenifoldError, Result};

use super::runner::AgentSessionConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentResponse {
    pub text: String,
    pub error: Option<String>,
}

impl From<AcpPromptResult> for AgentResponse {
    fn from(result: AcpPromptResult) -> Self {
        let error = if result.ok {
            None
        } else if !result.output.trim().is_empty() {
            Some(result.output.clone())
        } else {
            result.error_code.clone()
        };

        Self {
            text: result.output,
            error,
        }
    }
}

pub(crate) struct AgentSession {
    agent_name: String,
    runtime_kind: String,
    /// Internal pool key (app_session_id) used for CONN_MAP routing.
    session_id: String,
    /// Real ACP session ID returned by the agent process after the first prompt.
    real_acp_session_id: std::sync::Mutex<Option<String>>,
    cwd: String,
    model: Option<String>,
    thought_level: Option<String>,
    role_mode: Option<String>,
    config_options: Vec<(String, String)>,
    auto_approve: bool,
    event_tx: jockey::EventSender,
}

pub(crate) async fn open_agent_session(
    agent_name: &str,
    session_config: &AgentSessionConfig,
    event_tx: &jockey::EventSender,
) -> Result<AgentSession> {
    let cwd_str = session_config.cwd.to_string_lossy().to_string();
    let runtime_kind = resolve_runtime_kind(agent_name, &cwd_str)?;
    let auto_approve = session_config.auto_approve;
    let (_config_options, _modes) =
        jockey::acp::prewarm_role_for_config(&runtime_kind, agent_name, &cwd_str, None).await;
    let config_options = build_role_config_options(
        session_config.config_options.as_ref(),
        session_config.model.as_deref(),
        session_config.thought_level.as_deref(),
    )?;
    let session_id = uuid::Uuid::new_v4().to_string();
    info!(
        agent_name,
        cwd = %cwd_str,
        model = ?session_config.model,
        thought_level = ?session_config.thought_level,
        auto_approve,
        config_options_count = config_options.len(),
        %session_id,
        "opening pipeline agent session"
    );
    if !config_options.is_empty() {
        info!(
            %session_id,
            config_options = ?config_options,
            "agent session config options to apply"
        );
    }

    Ok(AgentSession {
        agent_name: agent_name.to_string(),
        runtime_kind,
        session_id,
        real_acp_session_id: std::sync::Mutex::new(None),
        cwd: cwd_str,
        model: session_config.model.clone(),
        thought_level: session_config.thought_level.clone(),
        role_mode: session_config.mode.clone(),
        config_options,
        auto_approve,
        event_tx: event_tx.clone(),
    })
}

impl AgentSession {
    /// Returns the real ACP session ID if captured from a prompt response,
    /// otherwise falls back to the internal pool key.
    pub(crate) fn remote_id(&self) -> String {
        self.real_acp_session_id
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| self.session_id.clone())
    }

    pub(crate) fn keep_alive(&self) {}

    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AgentResponse> {
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            prompt_len = prompt.len(),
            config_options_count = self.config_options.len(),
            model = ?self.model,
            thought_level = ?self.thought_level,
            "sending prompt to ACP agent"
        );
        let result = jockey::acp::execute_runtime(
            &self.runtime_kind,
            &self.agent_name,
            prompt,
            &[],
            &self.cwd,
            &self.event_tx,
            self.auto_approve,
            vec![],
            self.role_mode.clone(),
            self.config_options.clone(),
            None,
            &self.session_id,
        )
        .await;
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            ok = result.ok,
            output_len = result.output.len(),
            "ACP agent responded"
        );
        if !result.ok {
            warn!(
                agent = %self.agent_name,
                session_id = %self.session_id,
                output_preview = %result.output.chars().take(200).collect::<String>(),
                "ACP agent prompt failed"
            );
        }

        // Capture the real ACP session ID from the worker's response metadata.
        if let Some(sid) = result.meta.get("sessionId").and_then(|v| v.as_str()) {
            if let Ok(mut guard) = self.real_acp_session_id.lock() {
                *guard = Some(sid.to_string());
            }
        }

        Ok(result.into())
    }

    pub(crate) async fn close(self) -> Result<()> {
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            "closing pipeline agent session"
        );
        jockey::acp::reset_session(
            &self.runtime_kind,
            &self.agent_name,
            Some(self.session_id.as_str()),
        )
        .await
        .map_err(MaenifoldError::Message)?;
        Ok(())
    }
}

fn resolve_runtime_kind(agent_name: &str, cwd: &str) -> Result<String> {
    let config = AgentsConfig::load(std::path::Path::new(cwd));
    config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::AgentSetup(format!(
            "agent '{agent_name}' not configured in maenifold.json"
        ))
    })?;
    Ok(agent_name.to_string())
}

fn build_role_config_options(
    config_options: Option<&Map<String, Value>>,
    model: Option<&str>,
    thought_level: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let mut options = Vec::new();

    if let Some(map) = config_options {
        for (key, value) in map {
            options.push((key.clone(), config_value_to_string(key, value)?));
        }
    }

    push_option_if_present(&mut options, "model", model);
    push_thought_level_options(&mut options, thought_level);

    Ok(options)
}

fn push_option_if_present(options: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };

    if let Some(existing) = options
        .iter_mut()
        .find(|(existing_key, _)| existing_key == key)
    {
        existing.1 = value.to_string();
    } else {
        options.push((key.to_string(), value.to_string()));
    }
}

fn push_thought_level_options(options: &mut Vec<(String, String)>, thought_level: Option<&str>) {
    let Some(thought_level) = thought_level else {
        return;
    };

    push_option_if_present(options, "thought_level", Some(thought_level));
    push_option_if_present(options, "thoughtLevel", Some(thought_level));
}

fn config_value_to_string(key: &str, value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Null => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must not be null"
        ))),
        other => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must be a scalar (string, bool, or number), got {}",
            match other {
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "unknown",
            }
        ))),
    }
}
