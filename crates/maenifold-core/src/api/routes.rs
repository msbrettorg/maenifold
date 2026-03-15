use crate::acp::config::{AgentInfo, AgentsConfig};
use crate::api::graph;
use crate::{Config, db};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone)]
pub struct ApiState {
    config: Config,
    agents_config: AgentsConfig,
}

impl ApiState {
    #[must_use]
    pub fn new(config: Config, agents_config: AgentsConfig) -> Self {
        Self {
            config,
            agents_config,
        }
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(error: impl Display) -> Self {
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

fn parse_optional<T>(params: &HashMap<String, String>, key: &str) -> ApiResult<Option<T>>
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

async fn graph_nodes(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Vec<graph::GraphNode>>> {
    let community = parse_optional::<i64>(&params, "community")?;
    let center = params.get("center").map(String::as_str);
    let depth = parse_optional::<u32>(&params, "depth")?;
    let connection =
        db::open_connection(&state.config.database_path).map_err(ApiError::internal)?;
    let nodes =
        graph::get_nodes(&connection, community, center, depth).map_err(ApiError::internal)?;
    Ok(Json(nodes))
}

async fn graph_edges(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Vec<graph::GraphEdge>>> {
    let community = parse_optional::<i64>(&params, "community")?;
    let connection =
        db::open_connection(&state.config.database_path).map_err(ApiError::internal)?;
    let edges = graph::get_edges(&connection, community).map_err(ApiError::internal)?;
    Ok(Json(edges))
}

async fn list_agents(State(state): State<ApiState>) -> Json<Vec<AgentInfo>> {
    Json(state.agents_config.list_agents())
}

#[must_use]
pub fn api_routes(state: ApiState) -> Router {
    Router::new()
        .route("/api/graph/nodes", get(graph_nodes))
        .route("/api/graph/edges", get(graph_edges))
        .route("/api/agents", get(list_agents))
        .with_state(state)
}
