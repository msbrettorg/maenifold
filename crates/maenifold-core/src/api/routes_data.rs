use super::routes::{ApiError, ApiResult, ApiState, parse_optional};
use crate::api::graph;
use crate::db;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};

pub(super) async fn graph_nodes(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Vec<graph::GraphNode>>> {
    let community = parse_optional::<i64>(&params, "community")?;
    let center = params.get("center").map(String::as_str);
    let depth = parse_optional::<u32>(&params, "depth")?;
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let nodes =
        graph::get_nodes(&connection, community, center, depth).map_err(ApiError::internal)?;
    Ok(Json(nodes))
}

pub(super) async fn graph_edges(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Vec<graph::GraphEdge>>> {
    let community = parse_optional::<i64>(&params, "community")?;
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let edges = graph::get_edges(&connection, community).map_err(ApiError::internal)?;
    Ok(Json(edges))
}

#[derive(Deserialize)]
pub(super) struct FileTextQuery {
    path: String,
    cwd: Option<String>,
}

#[derive(Serialize)]
pub(super) struct FileTextResponse {
    path: String,
    content: String,
}

pub(crate) fn resolve_requested_file_path(path: &str, cwd: Option<&str>) -> ApiResult<PathBuf> {
    let requested = PathBuf::from(path);

    if requested.is_absolute() {
        return Ok(requested);
    }

    let cwd = cwd.ok_or_else(|| {
        ApiError::bad_request("relative file paths require a cwd query parameter")
    })?;
    let resolved = FsPath::new(cwd).join(requested);

    if resolved.is_absolute() {
        Ok(resolved)
    } else {
        Err(ApiError::bad_request("resolved file path must be absolute"))
    }
}

pub(super) async fn file_text(
    Query(params): Query<FileTextQuery>,
) -> ApiResult<Json<FileTextResponse>> {
    let resolved_path = resolve_requested_file_path(&params.path, params.cwd.as_deref())?;

    match std::fs::read_to_string(&resolved_path) {
        Ok(content) => Ok(Json(FileTextResponse {
            path: resolved_path.to_string_lossy().into_owned(),
            content,
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(ApiError::not_found(
            format!("File not found: {}", resolved_path.to_string_lossy()),
        )),
        Err(error) => Err(ApiError::internal(error)),
    }
}
