use std::sync::atomic::AtomicBool;
use tracing::info;

use crate::acp_config::AgentsConfig;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};

use super::agent_session::AgentSession;
use super::agent_session_config::build_role_config_options;
use super::agent_session_resume::{PromptGate, ResumeState};
use super::runner::AgentSessionConfig;

pub(crate) async fn open_agent_session(
    agent_name: &str,
    session_config: &AgentSessionConfig,
    event_tx: &jockey::EventSender,
) -> Result<AgentSession> {
    let cwd_str = session_config.cwd.to_string_lossy().to_string();
    let trusted_root = Config::current().maenifold_root;
    let runtime_kind = resolve_runtime_kind(agent_name, &trusted_root)?;
    let auto_approve = session_config.auto_approve;
    let session_id = session_config
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let actual_acp_session_id =
        prewarm_acp_session(agent_name, &runtime_kind, &cwd_str, &session_id).await?;
    let config_options = build_role_config_options(
        session_config.config_options.as_ref(),
        session_config.model.as_deref(),
        session_config.thought_level.as_deref(),
    )?;
    info!(
        agent_name,
        cwd = %cwd_str,
        model = ?session_config.model,
        thought_level = ?session_config.thought_level,
        auto_approve,
        config_options_count = config_options.len(),
        %session_id,
        acp_session_id = %actual_acp_session_id,
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
        real_acp_session_id: std::sync::Mutex::new(Some(actual_acp_session_id)),
        on_acp_session_captured: None,
        cwd: cwd_str,
        model: session_config.model.clone(),
        thought_level: session_config.thought_level.clone(),
        role_mode: session_config.mode.clone(),
        config_options,
        auto_approve,
        resume_state: ResumeState::new(session_config.resume_session_id.clone()),
        prompt_gate: PromptGate::new(),
        event_tx: event_tx.clone(),
        cleanup_on_drop: AtomicBool::new(true),
    })
}

async fn prewarm_acp_session(
    agent_name: &str,
    runtime_kind: &str,
    cwd: &str,
    session_id: &str,
) -> Result<String> {
    let (_config_options, _modes, actual_acp_session_id) =
        jockey::acp::prewarm_role_for_config(runtime_kind, agent_name, cwd, None, Some(session_id))
            .await;
    if actual_acp_session_id.trim().is_empty() {
        return Err(MaenifoldError::AgentSetup(format!(
            "agent '{agent_name}' did not return an ACP session id during session/new"
        )));
    }

    Ok(actual_acp_session_id)
}

fn resolve_runtime_kind(agent_name: &str, config_root: &std::path::Path) -> Result<String> {
    let config = AgentsConfig::load(config_root)?;
    config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::AgentSetup(format!(
            "agent '{agent_name}' not configured in maenifold.json"
        ))
    })?;
    Ok(agent_name.to_string())
}
