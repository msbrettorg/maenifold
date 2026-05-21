use crate::api::config_manager::ConfigManager;
use crate::api::config_watch::ConfigWatchState;
use crate::api::{
    routes_config::{
        config_watch_handler, get_config_handler, get_config_path_handler,
        machine_id_handler, reload_config_handler,
    },
};
use crate::mcp::PendingRequestRegistry;
use crate::mcp::stdio_pool::StdioPool;
use crate::Config;
use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use parking_lot::RwLock;
use serde_json::json;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Clone)]
#[allow(dead_code)]
pub struct ApiState {
    config: Arc<RwLock<Config>>,
    pub(super) config_manager: ConfigManager,
    pub(super) config_watch: ConfigWatchState,
    pub(crate) gateway_pool: Option<Arc<StdioPool>>,
    pub(crate) user_prompt_registry: Arc<PendingRequestRegistry>,
}

impl ApiState {
    #[must_use]
    #[allow(dead_code)]
    pub fn new(
        shared: impl Into<ApiStateSharedConfig>,
        gateway_pool: Option<Arc<StdioPool>>,
    ) -> Self {
        Self::with_user_prompt_registry(
            shared,
            gateway_pool,
            Arc::new(PendingRequestRegistry::new()),
        )
    }

    #[must_use]
    pub fn with_user_prompt_registry(
        shared: impl Into<ApiStateSharedConfig>,
        gateway_pool: Option<Arc<StdioPool>>,
        user_prompt_registry: Arc<PendingRequestRegistry>,
    ) -> Self {
        let shared = shared.into();
        let config_watch =
            ConfigWatchState::new(shared.config_manager.clone(), Arc::clone(&shared.config));
        Self {
            config: shared.config,
            config_manager: shared.config_manager,
            config_watch,
            gateway_pool,
            user_prompt_registry,
        }
    }

}

pub struct ApiStateSharedConfig {
    config: Arc<RwLock<Config>>,
    config_manager: ConfigManager,
}

impl From<Config> for ApiStateSharedConfig {
    fn from(config: Config) -> Self {
        let config = Arc::new(RwLock::new(config));
        let config_manager = {
            let config_guard = config.read();
            ConfigManager::new(&config_guard)
        };
        Self {
            config,
            config_manager,
        }
    }
}

pub(crate) type ApiResult<T> = std::result::Result<T, ApiError>;

pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub(crate) fn internal(error: impl Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

pub fn api_routes(state: ApiState) -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/config", get(get_config_handler))
        .route("/api/config/path", get(get_config_path_handler))
        .route("/api/config/reload", post(reload_config_handler))
        .route("/api/config/watch", get(config_watch_handler))
        .route("/api/machine-id", get(machine_id_handler))
        .route(
            "/api/user-prompt/stream",
            get(crate::api::user_prompt::stream_handler),
        )
        .route(
            "/api/user-prompt/respond",
            post(crate::api::user_prompt::respond_handler),
        )
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
