use super::acp_relay::RelayConfig;
use super::acp_relay_helpers::rpc_error;
use crate::acp_config::AgentsConfig;
use agent_client_protocol as acp;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::sync::oneshot;

pub(super) async fn reset_session_connection(
    relay: &RelayConfig,
    app_session_id: &str,
    request_id: Value,
) -> Result<(), Value> {
    let (reset_tx, reset_rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Reset {
            runtime_key: relay.adapter.runtime_key.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.to_string(),
            result_tx: reset_tx,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    reset_rx
        .await
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?
        .map_err(|error| rpc_error(request_id, -32000, error))
}

pub(super) async fn prewarm_session(
    relay: &RelayConfig,
    app_session_id: &str,
    cwd: String,
    mcp_servers: Vec<acp::McpServer>,
    client_capabilities: acp::ClientCapabilities,
    resume_session_id: Option<String>,
    config_options: HashMap<String, Value>,
    request_id: Value,
) -> Result<(Vec<Value>, Vec<String>, String), Value> {
    let role_config_options = role_config_options(config_options)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Prewarm {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            client_capabilities,
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.to_string(),
            cwd,
            // Design decision: auto_approve is always true for the single-user desktop app.
            // The user is the operator — click-through permissions on every fs/write would
            // make pipelines unusable. When multi-user support is needed, this should be
            // configurable via relay settings.
            auto_approve: true,
            mcp_servers,
            role_mode: None,
            role_config_options,
            delta_tx: None,
            result_tx: Some(tx),
            resume_session_id,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    rx.await
        .map_err(|_| rpc_error(request_id, -32000, "ACP worker disconnected"))
}

fn role_config_options(config_options: HashMap<String, Value>) -> Result<Vec<Value>, String> {
    config_options
        .into_iter()
        .map(|(key, value)| normalize_role_config_option(&key, value))
        .collect()
}

fn config_value_to_json(key: &str, value: Value) -> Result<Value, String> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => Ok(value),
        Value::Null => Err(format!("configOptions.{key} must not be null")),
        other => Err(format!(
            "configOptions.{key} must be a scalar (string, bool, or number), got {}",
            match &other {
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "unknown",
            }
        )),
    }
}

fn normalize_role_config_option(key: &str, value: Value) -> Result<Value, String> {
    match value {
        Value::Object(mut option) => {
            if let Some(id) = option.get("id").and_then(Value::as_str) {
                if id != key {
                    return Err(format!(
                        "configOptions.{key}.id must match the config option key"
                    ));
                }
            } else {
                option.insert("id".to_string(), Value::String(key.to_string()));
            }

            option
                .entry("name".to_string())
                .or_insert_with(|| Value::String(key.to_string()));

            if !option.contains_key("currentValue") {
                return Err(format!(
                    "configOptions.{key} objects must include currentValue"
                ));
            }

            Ok(Value::Object(option))
        }
        other => {
            let current_value = config_value_to_json(key, other)?;
            let option_name = current_value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| current_value.to_string());
            let mut option = serde_json::Map::from_iter([
                ("id".to_string(), Value::String(key.to_string())),
                ("name".to_string(), Value::String(key.to_string())),
                ("currentValue".to_string(), current_value.clone()),
            ]);
            if current_value.is_boolean() {
                option.insert("type".to_string(), Value::String("boolean".to_string()));
            } else {
                option.insert("type".to_string(), Value::String("select".to_string()));
                option.insert(
                    "options".to_string(),
                    Value::Array(vec![json!({
                        "value": current_value,
                        "name": option_name,
                    })]),
                );
            }
            Ok(Value::Object(option))
        }
    }
}
/// Load configured MCP servers (global + per-agent) from maenifold.json,
/// deduplicated by name (case-insensitive), and converted to ACP schema.
pub(super) fn load_configured_mcp_servers(root: &Path, agent_name: &str) -> Vec<acp::McpServer> {
    let config = AgentsConfig::load(root);
    let agent_servers = config
        .agents
        .get(agent_name)
        .map(|entry| entry.mcp_servers.as_slice())
        .unwrap_or(&[]);

    let mut seen = HashSet::new();
    let mut servers = Vec::new();
    for entry in config.mcp_servers.iter().chain(agent_servers.iter()) {
        let name = entry.name().to_ascii_lowercase();
        if seen.insert(name) {
            servers.push(entry.to_acp());
        }
    }
    servers
}

/// Merge configured MCP servers with client-provided ones.
/// Configured servers take precedence — client-provided servers are only
/// added when no configured server with the same name exists.
pub(super) fn merge_mcp_servers(
    configured: &[acp::McpServer],
    client_provided: Vec<acp::McpServer>,
) -> Vec<acp::McpServer> {
    let mut servers = configured.to_vec();
    let existing: HashSet<String> = servers
        .iter()
        .filter_map(mcp_server_name)
        .map(|name| name.to_ascii_lowercase())
        .collect();
    for server in client_provided {
        let dominated = mcp_server_name(&server)
            .map(|name| existing.contains(&name.to_ascii_lowercase()))
            .unwrap_or(false);
        if !dominated {
            servers.push(server);
        }
    }
    servers
}

fn mcp_server_name(server: &acp::McpServer) -> Option<&str> {
    match server {
        acp::McpServer::Http(s) => Some(s.name.as_str()),
        acp::McpServer::Sse(s) => Some(s.name.as_str()),
        acp::McpServer::Stdio(s) => Some(s.name.as_str()),
        _ => None,
    }
}
