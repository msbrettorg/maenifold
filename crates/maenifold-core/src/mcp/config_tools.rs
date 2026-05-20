#![cfg_attr(test, allow(dead_code))]

use crate::acp_config::{AgentEntry, AgentsConfig, MaenifoldSettings, McpServerEntry};
use crate::api::config_manager::ConfigManager;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use jockey::acp::clear_adapter_cache;
use parking_lot::RwLock;
use rmcp::model::{CallToolResult, Content};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::mcp::resources::notify_resource_updated;

const CONFIG_SETTINGS_URI: &str = "config://settings";
const CONFIG_AGENTS_URI: &str = "config://agents";
const CONFIG_MCP_SERVERS_URI: &str = "config://mcpServers";

const CONFIG_MUTATION_TOOLS: &[&str] = &[
    "ReloadConfig",
    "UpdateSettings",
    "AddAgent",
    "UpdateAgent",
    "DeleteAgent",
    "AddMcpServer",
    "UpdateMcpServer",
    "DeleteMcpServer",
];

#[derive(Clone)]
pub(crate) struct ConfigMutationState {
    config_manager: ConfigManager,
    runtime_config: Arc<RwLock<Config>>,
}

impl ConfigMutationState {
    pub(crate) fn new(config_manager: ConfigManager, runtime_config: Arc<RwLock<Config>>) -> Self {
        Self {
            config_manager,
            runtime_config,
        }
    }

    pub(crate) fn config_manager(&self) -> ConfigManager {
        self.config_manager.clone()
    }

    pub(crate) fn runtime_config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.runtime_config)
    }

    fn refresh_settings(&self, agents_config: &AgentsConfig) {
        self.runtime_config.write().settings = agents_config.settings.clone().unwrap_or_default();
    }

    async fn emit_resource_notifications(&self, uris: &[&str]) {
        for uri in uris {
            notify_resource_updated(uri).await;
        }
    }
}

pub(crate) fn mutation_state(
    config: &Config,
    config_manager: ConfigManager,
) -> ConfigMutationState {
    ConfigMutationState::new(
        config_manager,
        Arc::new(parking_lot::RwLock::new(config.clone())),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReloadConfigResponse {
    config: AgentsConfig,
    reloaded: Vec<&'static str>,
    requires_restart: Vec<&'static str>,
}

#[derive(Deserialize)]
struct SettingsRequest {
    settings: Value,
}

#[derive(Deserialize)]
struct AgentRequest {
    name: String,
    config: AgentEntry,
}

#[derive(Deserialize)]
struct NameRequest {
    name: String,
}

#[derive(Deserialize)]
struct McpServerRequest {
    name: String,
    entry: McpServerEntry,
}

#[derive(Deserialize)]
struct NamedMcpServerRequest {
    name: String,
    entry: McpServerEntry,
}

pub(crate) fn is_config_mutation_tool(tool_name: &str) -> bool {
    CONFIG_MUTATION_TOOLS.contains(&tool_name)
}

pub(crate) async fn handle_tool_call(
    state: ConfigMutationState,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let result = match tool_name {
        "ReloadConfig" => reload_config(&state),
        "UpdateSettings" => update_settings(&state, arguments),
        "AddAgent" => add_agent(&state, arguments),
        "UpdateAgent" => update_agent(&state, arguments),
        "DeleteAgent" => delete_agent(&state, arguments),
        "AddMcpServer" => add_mcp_server(&state, arguments),
        "UpdateMcpServer" => update_mcp_server(&state, arguments),
        "DeleteMcpServer" => delete_mcp_server(&state, arguments),
        _ => Err(rmcp::ErrorData::invalid_params(
            format!("Unknown config mutation tool: {tool_name}"),
            None,
        )),
    }?;
    state
        .emit_resource_notifications(notification_uris(tool_name))
        .await;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

fn notification_uris(tool_name: &str) -> &'static [&'static str] {
    match tool_name {
        "ReloadConfig" => &[
            CONFIG_SETTINGS_URI,
            CONFIG_AGENTS_URI,
            CONFIG_MCP_SERVERS_URI,
        ],
        "UpdateSettings" => &[CONFIG_SETTINGS_URI],
        "AddAgent" | "UpdateAgent" | "DeleteAgent" => &[CONFIG_AGENTS_URI],
        "AddMcpServer" | "UpdateMcpServer" | "DeleteMcpServer" => &[CONFIG_MCP_SERVERS_URI],
        _ => &[],
    }
}

pub(crate) fn registry_invocation_unavailable(
    _context: &ToolContext,
    _payload: &Value,
) -> Result<String> {
    Err(MaenifoldError::Message(
        "Config mutation tools require MCP HTTP daemon state and must be called through tools/call"
            .to_string(),
    ))
}

fn reload_config(state: &ConfigMutationState) -> std::result::Result<String, rmcp::ErrorData> {
    let config = state.config_manager.reload().map_err(internal_error)?;
    state.refresh_settings(&config);
    render(&ReloadConfigResponse {
        config,
        reloaded: vec!["configManagerState", "apiRuntimeSettings"],
        requires_restart: vec!["agentRuntimeBehavior"],
    })
}

fn update_settings(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: SettingsRequest = parse(arguments)?;
    let current = state
        .config_manager
        .get_config()
        .settings
        .unwrap_or_default();
    let mut merged = serde_json::to_value(current).map_err(internal_error)?;
    deep_merge(&mut merged, request.settings);
    let settings: MaenifoldSettings = serde_json::from_value(merged).map_err(invalid_params)?;
    let config = state
        .config_manager
        .update_settings(settings)
        .map_err(internal_error)?;
    state.refresh_settings(&config);
    state.config_manager.render_config().map_err(internal_error)
}

fn add_agent(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: AgentRequest = parse(arguments)?;
    let config = state
        .config_manager
        .add_agent(request.name, request.config)
        .map_err(internal_error)?;
    clear_adapter_cache();
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn update_agent(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: AgentRequest = parse(arguments)?;
    let config = state
        .config_manager
        .update_agent(request.name, request.config)
        .map_err(internal_error)?;
    clear_adapter_cache();
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn delete_agent(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: NameRequest = parse(arguments)?;
    let config = state
        .config_manager
        .remove_agent(&request.name)
        .map_err(internal_error)?;
    clear_adapter_cache();
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn add_mcp_server(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: McpServerRequest = parse(arguments)?;
    let config = state
        .config_manager
        .add_mcp_server(request.name, request.entry)
        .map_err(internal_error)?;
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn update_mcp_server(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: NamedMcpServerRequest = parse(arguments)?;
    let config = state
        .config_manager
        .update_mcp_server(&request.name, request.entry)
        .map_err(internal_error)?;
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn delete_mcp_server(
    state: &ConfigMutationState,
    arguments: Value,
) -> std::result::Result<String, rmcp::ErrorData> {
    let request: NameRequest = parse(arguments)?;
    let config = state
        .config_manager
        .remove_mcp_server(&request.name)
        .map_err(internal_error)?;
    let _ = config;
    state.config_manager.render_config().map_err(internal_error)
}

fn deep_merge(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target_map), Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                deep_merge(target_map.entry(key).or_insert(Value::Null), value);
            }
        }
        (target_slot, patch_value) => *target_slot = patch_value,
    }
}

fn parse<T: for<'de> Deserialize<'de>>(
    arguments: Value,
) -> std::result::Result<T, rmcp::ErrorData> {
    serde_json::from_value(arguments).map_err(invalid_params)
}

fn render<T: Serialize>(value: &T) -> std::result::Result<String, rmcp::ErrorData> {
    serde_json::to_string_pretty(value).map_err(internal_error)
}

fn invalid_params(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(error.to_string(), Some(json!(null)))
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), Some(json!(null)))
}
