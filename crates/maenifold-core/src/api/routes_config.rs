use super::routes::{ApiError, ApiResult, ApiState};
use crate::acp_config::AgentsConfig;
use crate::api::{config_watch, mcp_discovery};
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};

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

pub(super) async fn reload_config_handler(
    State(state): State<ApiState>,
) -> ApiResult<Json<serde_json::Value>> {
    let config = state.config_manager.reload().map_err(ApiError::internal)?;
    Ok(Json(serde_json::json!({
        "config": config,
        "reloaded": ["configManagerState", "apiRuntimeSettings"],
        "requiresRestart": ["agentRuntimeBehavior"],
    })))
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
    let mut results = Vec::new();

    if let Some(pool) = &state.gateway_pool {
        for (server_name, tools) in pool.all_gateway_tools() {
            let tools = tools
                .into_iter()
                .map(|tool| mcp_discovery::McpDiscoveredTool {
                    name: format!("{server_name}.{}", tool.name),
                    description: tool.description.as_ref().map(ToString::to_string),
                    input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                })
                .collect();
            results.push(mcp_discovery::McpServerDiscoveryResult {
                server: server_name,
                server_type: "stdio".to_string(),
                status: mcp_discovery::McpDiscoveryStatus::Ok,
                error: None,
                tools,
            });
        }
    }

    let non_gateway: Vec<_> = config
        .mcp_servers
        .iter()
        .filter(|(_name, server)| !server.is_gateway())
        .map(|(name, server)| (name.clone(), server.clone()))
        .collect();
    if !non_gateway.is_empty() {
        results.extend(
            mcp_discovery::discover_all_mcp_tools(
                &non_gateway,
                mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT,
            )
            .await,
        );
    }

    Ok(Json(results))
}
