use crate::api::graph;
use crate::markdown::{extract_h2_sections, file_exists, find_file_as_uri, read_markdown};
use crate::search::search_memories;
use crate::tools::{ToolContext, collect_recent_activity_records, parse_recent_activity_timespan};
use crate::util::json::OutputFormat;
use crate::{Config, db};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone)]
pub struct ApiState {
    config: Config,
}

impl ApiState {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self { config }
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

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
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

#[derive(Serialize)]
struct MemoryReadResponse {
    uri: String,
    title: String,
    content: String,
    created: Option<String>,
    modified: Option<String>,
}

async fn memory_read(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<MemoryReadResponse>> {
    let identifier = params
        .get("uri")
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("missing required 'uri' query parameter"))?;

    // Resolve identifier to a memory:// URI (supports both URIs and title lookups)
    let uri = if identifier.starts_with("memory://") {
        identifier.clone()
    } else {
        find_file_as_uri(identifier, &state.config)
            .ok_or_else(|| ApiError::not_found(format!("Memory file not found: {identifier}")))?
    };

    // Verify the file exists on disk
    if !file_exists(&uri, &state.config).map_err(ApiError::internal)? {
        return Err(ApiError::not_found(format!(
            "Memory file not found: {identifier}"
        )));
    }

    let (frontmatter, content, _checksum) =
        read_markdown(&uri, &state.config).map_err(ApiError::internal)?;

    let title = frontmatter
        .as_ref()
        .and_then(|fm| fm.get("title"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
                .to_string()
        });

    let created = frontmatter
        .as_ref()
        .and_then(|fm| fm.get("created"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let modified = frontmatter
        .as_ref()
        .and_then(|fm| fm.get("modified"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Json(MemoryReadResponse {
        uri,
        title,
        content,
        created,
        modified,
    }))
}

#[derive(Serialize)]
struct RecentActivityItem {
    uri: String,
    title: String,
    modified: String,
    sections: Vec<String>,
}

#[derive(Serialize)]
struct RecentActivityResponse {
    items: Vec<RecentActivityItem>,
}

async fn memory_recent(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<RecentActivityResponse>> {
    let filter = params.get("filter").map(String::as_str);
    let limit = parse_optional::<i64>(&params, "limit")?.unwrap_or(10);
    if limit <= 0 {
        return Err(ApiError::bad_request("limit must be a positive integer"));
    }
    let limit = limit.min(1_000);
    let include_content = parse_optional::<bool>(&params, "includeContent")?.unwrap_or(false);
    let timespan = params
        .get("timespan")
        .map(|v| parse_recent_activity_timespan(v).map_err(ApiError::internal))
        .transpose()?;

    let records = collect_recent_activity_records(&state.config, timespan, filter, limit)
        .map_err(ApiError::internal)?;

    let items = records
        .into_iter()
        .map(|record| {
            let sections = extract_h2_sections(&record.content)
                .into_iter()
                .map(|(heading, body)| {
                    if include_content {
                        format!("{heading}\n{body}")
                    } else {
                        heading
                    }
                })
                .collect();
            RecentActivityItem {
                uri: record.file_path,
                title: record.title,
                modified: record.last_indexed,
                sections,
            }
        })
        .collect();

    Ok(Json(RecentActivityResponse { items }))
}

#[derive(Serialize)]
struct SearchResultItem {
    uri: String,
    title: String,
    snippet: String,
    score: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySearchResponse {
    results: Vec<SearchResultItem>,
    total_count: u64,
    page: u64,
    page_size: u64,
}

async fn memory_search(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<MemorySearchResponse>> {
    let query = params
        .get("query")
        .map(|value| value.trim())
        .unwrap_or_default();
    if query.is_empty() {
        return Err(ApiError::bad_request(
            "query parameter is required and must not be empty",
        ));
    }

    let mode = params
        .get("mode")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("Hybrid");
    let page_size: u64 = parse_optional::<u64>(&params, "pageSize")?.unwrap_or(10);
    let page: u64 = parse_optional::<u64>(&params, "page")?.unwrap_or(1);
    if page_size == 0 {
        return Err(ApiError::bad_request("pageSize must be a positive integer"));
    }
    if page == 0 {
        return Err(ApiError::bad_request("page must be a positive integer"));
    }
    let folder = params
        .get("folder")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    let mut payload = json!({
        "query": query,
        "mode": mode,
        "pageSize": page_size,
        "page": page,
    });
    if let Some(f) = folder {
        payload["folder"] = json!(f);
    }

    let context = ToolContext::new(state.config.clone(), OutputFormat::Json);
    let raw = search_memories(&context, &payload).map_err(ApiError::internal)?;

    // search_memories with Json output wraps results in {"success":true,"data":{...}}
    let outer: Value = serde_json::from_str(&raw).map_err(|error| {
        if raw.starts_with("ERROR:") {
            ApiError::bad_request(raw.clone())
        } else {
            ApiError::internal(format!("search parse error: {error}"))
        }
    })?;
    let data = outer
        .get("data")
        .ok_or_else(|| ApiError::internal("unexpected search response shape"))?;

    // Choose the score field matching the requested mode
    let score_key = if mode.eq_ignore_ascii_case("semantic") {
        "semanticScore"
    } else if mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text") {
        "textScore"
    } else {
        "fusedScore"
    };

    let empty_vec: Vec<Value> = vec![];
    let raw_results = data
        .get("results")
        .and_then(Value::as_array)
        .unwrap_or(&empty_vec);

    let results: Vec<SearchResultItem> = raw_results
        .iter()
        .map(|r| SearchResultItem {
            uri: r
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            title: r
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            snippet: r
                .get("snippet")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            score: r.get(score_key).and_then(Value::as_f64).unwrap_or(0.0),
        })
        .collect();

    let total_count = data.get("totalCount").and_then(Value::as_u64).unwrap_or(0);
    let page_out = data.get("page").and_then(Value::as_u64).unwrap_or(page);
    let page_size_out = data
        .get("pageSize")
        .and_then(Value::as_u64)
        .unwrap_or(page_size);

    Ok(Json(MemorySearchResponse {
        results,
        total_count,
        page: page_out,
        page_size: page_size_out,
    }))
}

#[must_use]
pub fn api_routes(state: ApiState) -> Router {
    Router::new()
        .route("/api/graph/nodes", get(graph_nodes))
        .route("/api/graph/edges", get(graph_edges))
        .route("/api/memory/read", get(memory_read))
        .route("/api/memory/recent", get(memory_recent))
        .route("/api/memory/search", get(memory_search))
        .with_state(state)
}
