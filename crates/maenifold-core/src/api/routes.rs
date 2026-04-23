use crate::acp_config::AgentsConfig;
use crate::api::config_manager::ConfigManager;
use crate::api::config_watch::ConfigWatchState;
use crate::api::{
    routes_config::{
        add_agent_handler, add_mcp_server_handler, config_watch_handler, delete_agent_handler,
        delete_mcp_server_handler, discover_mcp_tools_handler, get_config_handler,
        get_config_path_handler, machine_id_handler, reload_config_handler, update_agent_handler,
        update_mcp_server_handler, update_settings_handler,
    },
    routes_data::{
        file_text, graph_communities, graph_community_concepts, graph_edges, graph_nodes,
    },
    routes_memory::{memory_read, memory_recent, memory_search},
};
use crate::{Config, db};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post, put},
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    config: Arc<RwLock<Config>>,
    pub(super) config_manager: ConfigManager,
    pub(super) config_watch: ConfigWatchState,
}

impl ApiState {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let config = Arc::new(RwLock::new(config));
        let config_manager = {
            let config_guard = config.read();
            ConfigManager::new(&config_guard)
        };
        let config_watch = ConfigWatchState::new(config_manager.clone(), Arc::clone(&config));
        Self {
            config,
            config_manager,
            config_watch,
        }
    }

    pub(super) fn config(&self) -> Config {
        self.config.read().clone()
    }

    pub(super) fn refresh_settings(&self, agents_config: &AgentsConfig) {
        self.config.write().settings = agents_config.settings.clone().unwrap_or_default();
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

pub(super) fn parse_optional<T>(
    params: &HashMap<String, String>,
    key: &str,
) -> ApiResult<Option<T>>
where
    T: FromStr,
    T::Err: Display,
{
    params
        .get(key)
        .map(|value| {
            value.parse::<T>().map_err(|error| {
                ApiError::bad_request(format!(
                    "invalid '{key}' query parameter '{value}': {error}"
                ))
            })
        })
        .transpose()
}

pub fn api_routes(state: ApiState) -> Router {
    Router::new()
        .route("/api/config", get(get_config_handler))
        .route("/api/config/path", get(get_config_path_handler))
        .route("/api/config/watch", get(config_watch_handler))
        .route("/api/config/reload", post(reload_config_handler))
        .route("/api/machine-id", get(machine_id_handler))
        .route("/api/config/agents", post(add_agent_handler))
        .route(
            "/api/config/agents/{name}",
            put(update_agent_handler).delete(delete_agent_handler),
        )
        .route("/api/config/mcp-servers", post(add_mcp_server_handler))
        .route(
            "/api/config/mcp-servers/{name}",
            put(update_mcp_server_handler).delete(delete_mcp_server_handler),
        )
        .route("/api/config/settings", post(update_settings_handler))
        .route("/api/files/text", get(file_text))
        .route("/api/graph/communities", get(graph_communities))
        .route(
            "/api/graph/communities/{id}/concepts",
            get(graph_community_concepts),
        )
        .route("/api/graph/nodes", get(graph_nodes))
        .route("/api/graph/edges", get(graph_edges))
        .route("/api/memory/read", get(memory_read))
        .route("/api/memory/recent", get(memory_recent))
        .route("/api/memory/search", get(memory_search))
        .route("/api/discovery/mcp-tools", get(discover_mcp_tools_handler))
        .route("/api/pipeline/stream", get(pipeline_stream_handler))
        .with_state(state)
}

// T-92.7: RTM FR-36 — Endophasia frontend + API routes for /endophasia,
// /api/endophasia/communities, and /api/endophasia/stream.
// Kept separate from api_routes so serve_http can mount these without
// pulling in the full REST surface.

async fn endophasia_frontend() -> axum::response::Response {
    crate::endophasia::frontend::index_html_response()
}

async fn endophasia_communities_handler(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<crate::api::endophasia_communities::EndophasiaCommunitiesResponse>> {
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let response = crate::api::endophasia_communities::load_response(&connection, &params)
        .map_err(ApiError::internal)?;
    Ok(Json(response))
}

async fn endophasia_stream_handler() -> impl IntoResponse {
    crate::api::endophasia_stream::endophasia_stream().await
}

async fn pipeline_stream_handler() -> impl IntoResponse {
    crate::api::pipeline_stream::pipeline_stream().await
}

/// Routes for the Endophasia visualization UI and its backing API.
///
/// Mounts:
/// - `GET /endophasia`                         — embedded SPA (`index.html`)
/// - `GET /api/endophasia/communities`          — concept-community graph JSON
/// - `GET /api/endophasia/stream`               — SSE stream of `ConceptEvent`s
pub fn endophasia_routes(state: ApiState) -> Router {
    Router::new()
        .route("/endophasia", get(endophasia_frontend))
        .route(
            "/api/endophasia/communities",
            get(endophasia_communities_handler),
        )
        .route("/api/endophasia/stream", get(endophasia_stream_handler))
        .with_state(state)
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
