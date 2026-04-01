use super::context::ToolContext;
use crate::acp_config::AgentsConfig;
use crate::acp_native::{AgentError, SessionOpts, spawn_agent};
use crate::error::{MaenifoldError, Result};
use crate::workflow::pipeline::SUPERVISOR_RUNTIME;
use serde_json::Value;

/// MCP tool: spawn an ACP agent, create a session, send a single prompt, and
/// return the agent's response text.  Bridges sync→async via the shared
/// `SUPERVISOR_RUNTIME`.
pub fn acp_prompt(context: &ToolContext, payload: &Value) -> Result<String> {
    let agent_name = payload
        .get("agent")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("agent"))?;
    let prompt = payload
        .get("prompt")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("prompt"))?;

    let model = payload.get("model").and_then(Value::as_str).map(String::from);
    let mode = payload.get("mode").and_then(Value::as_str).map(String::from);
    let timeout_seconds = payload
        .get("timeoutSeconds")
        .and_then(Value::as_u64);

    let root = &context.config.maenifold_root;
    let config = AgentsConfig::load(root);
    let entry = config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!(
            "agent '{agent_name}' not configured in agents.json"
        ))
    })?;

    let command = entry.command.clone();
    let env = entry.env.clone();
    let mcp_servers: Vec<_> = entry.mcp_servers.iter().map(|s| s.to_acp()).collect();
    let cwd = root.clone();

    let prompt_text = prompt.to_string();
    let result = SUPERVISOR_RUNTIME.block_on(async move {
        let handle = spawn_agent(command, cwd.clone(), env);

        let session_info = handle
            .new_session(SessionOpts {
                cwd,
                model,
                mode,
                thought_level: None,
                config_options: None,
                allowed_tools: None,
                max_turns: None,
                timeout_seconds,
                mcp_servers,
            })
            .await?;

        let session_id = session_info.session_id;
        let response = handle.prompt(&session_id, &prompt_text).await;
        handle.close_session(&session_id).await;

        response
    });

    match result {
        Ok(response) => Ok(response.text),
        Err(AgentError::NotFound(message)) => Err(MaenifoldError::InvalidArgument(message)),
        Err(error) => Err(MaenifoldError::Message(error.to_string())),
    }
}
