use super::routes::{ApiError, ApiResult, ApiState};
use crate::acp_config::{AgentEntry, AgentsConfig, MaenifoldSettings, McpServerEntry};
use crate::api::{config_watch, mcp_discovery};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(super) struct AddAgentRequest {
    name: String,
    config: AgentEntry,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum AgentPayload {
    Wrapped { config: AgentEntry },
    Raw(AgentEntry),
}

impl AgentPayload {
    fn into_entry(self) -> AgentEntry {
        match self {
            AgentPayload::Wrapped { config } | AgentPayload::Raw(config) => config,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum McpServerPayload {
    Wrapped { entry: McpServerEntry },
    Raw(McpServerEntry),
}

impl McpServerPayload {
    fn into_entry(self) -> McpServerEntry {
        match self {
            McpServerPayload::Wrapped { entry } | McpServerPayload::Raw(entry) => entry,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum SettingsPayload {
    Wrapped { settings: MaenifoldSettings },
    Raw(MaenifoldSettings),
}

impl SettingsPayload {
    fn into_settings(self) -> MaenifoldSettings {
        match self {
            SettingsPayload::Wrapped { settings } | SettingsPayload::Raw(settings) => settings,
        }
    }
}

fn not_found_or_internal(error: String) -> ApiError {
    if error.contains("not found") {
        ApiError::not_found(error)
    } else {
        ApiError::internal(error)
    }
}

pub(super) async fn get_config_handler(
    State(state): State<ApiState>,
) -> ApiResult<Json<AgentsConfig>> {
    Ok(Json(state.config_manager.get_config()))
}

pub(super) async fn get_config_path_handler(
    State(state): State<ApiState>,
) -> ApiResult<Json<String>> {
    Ok(Json(
        state
            .config_manager
            .get_config_path()
            .to_string_lossy()
            .to_string(),
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReloadConfigResponse {
    config: AgentsConfig,
    reloaded: Vec<&'static str>,
    requires_restart: Vec<&'static str>,
}

pub(super) async fn reload_config_handler(
    State(state): State<ApiState>,
) -> ApiResult<Json<ReloadConfigResponse>> {
    let config = state.config_manager.reload().map_err(ApiError::internal)?;
    state.refresh_settings(&config);
    Ok(Json(ReloadConfigResponse {
        config,
        reloaded: vec!["configManagerState", "apiRuntimeSettings"],
        requires_restart: vec!["agentRuntimeBehavior"],
    }))
}

pub(super) async fn config_watch_handler(
    State(state): State<ApiState>,
) -> ApiResult<impl IntoResponse> {
    if let Some(error) = state.config_watch.startup_error() {
        return Err(ApiError::internal(format!(
            "config watch is unavailable: {error}"
        )));
    }

    Ok(config_watch::config_watch_stream(
        state.config_watch.clone(),
        state.config_manager.get_config_path(),
    )
    .await)
}

pub(super) async fn machine_id_handler() -> ApiResult<Json<String>> {
    machine_uid::get()
        .map(Json)
        .map_err(|error| ApiError::internal(format!("Failed to get machine ID: {error}")))
}

pub(super) async fn discover_mcp_tools_handler(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<mcp_discovery::McpServerDiscoveryResult>>> {
    let config = state.config_manager.get_config();
    Ok(Json(
        mcp_discovery::discover_all_mcp_tools(
            &config.mcp_servers,
            mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT,
        )
        .await,
    ))
}

pub(super) async fn add_agent_handler(
    State(state): State<ApiState>,
    Json(request): Json<AddAgentRequest>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .add_agent(request.name, request.config)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

pub(super) async fn update_agent_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(payload): Json<AgentPayload>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .update_agent(name, payload.into_entry())
        .map_err(not_found_or_internal)?;
    Ok(Json(config))
}

pub(super) async fn delete_agent_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .remove_agent(&name)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

pub(super) async fn add_mcp_server_handler(
    State(state): State<ApiState>,
    Json(payload): Json<McpServerPayload>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .add_mcp_server(payload.into_entry())
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

pub(super) async fn update_mcp_server_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(payload): Json<McpServerPayload>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .update_mcp_server(&name, payload.into_entry())
        .map_err(not_found_or_internal)?;
    Ok(Json(config))
}

pub(super) async fn delete_mcp_server_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .remove_mcp_server(&name)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

pub(super) async fn update_settings_handler(
    State(state): State<ApiState>,
    Json(payload): Json<SettingsPayload>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .update_settings(payload.into_settings())
        .map_err(ApiError::internal)?;
    state.refresh_settings(&config);
    Ok(Json(config))
}
