use super::routes::{ApiError, ApiResult, ApiState};
use crate::acp_config::AgentsConfig;
use crate::api::config_watch;
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
