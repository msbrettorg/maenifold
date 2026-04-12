use crate::acp_config::{AgentEntry, AgentsConfig, MaenifoldSettings, McpServerEntry};
use crate::api::config_manager::ConfigManager;
use crate::api::config_watch::ConfigWatchState;
use crate::api::graph;
use crate::api::{config_watch, mcp_discovery};
use crate::markdown::{extract_h2_sections, file_exists, find_file_as_uri, read_markdown};
use crate::search::search_memories;
use crate::tools::{ToolContext, collect_recent_activity_records, parse_recent_activity_timespan};
use crate::util::json::OutputFormat;
use crate::{Config, db};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post, put},
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path as FsPath, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    config: Arc<RwLock<Config>>,
    config_manager: ConfigManager,
    config_watch: ConfigWatchState,
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

    fn config(&self) -> Config {
        self.config.read().clone()
    }

    fn refresh_settings(&self, agents_config: &AgentsConfig) {
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
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let nodes =
        graph::get_nodes(&connection, community, center, depth).map_err(ApiError::internal)?;
    Ok(Json(nodes))
}

async fn graph_edges(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Vec<graph::GraphEdge>>> {
    let community = parse_optional::<i64>(&params, "community")?;
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let edges = graph::get_edges(&connection, community).map_err(ApiError::internal)?;
    Ok(Json(edges))
}

async fn graph_communities(
    State(state): State<ApiState>,
) -> ApiResult<Json<graph::CommunityGraph>> {
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let communities = graph::get_communities(&connection).map_err(ApiError::internal)?;
    Ok(Json(communities))
}

async fn graph_community_concepts(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<graph::ConceptGraph>> {
    let config = state.config();
    let connection = db::open_connection(&config.database_path).map_err(ApiError::internal)?;
    let concepts = graph::get_community_concepts(&connection, id).map_err(ApiError::internal)?;
    Ok(Json(concepts))
}

#[derive(Deserialize)]
struct FileTextQuery {
    path: String,
    cwd: Option<String>,
}

#[derive(Serialize)]
struct FileTextResponse {
    path: String,
    content: String,
}

fn resolve_requested_file_path(path: &str, cwd: Option<&str>) -> ApiResult<PathBuf> {
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

async fn file_text(Query(params): Query<FileTextQuery>) -> ApiResult<Json<FileTextResponse>> {
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
        find_file_as_uri(identifier, &state.config())
            .ok_or_else(|| ApiError::not_found(format!("Memory file not found: {identifier}")))?
    };

    // Verify the file exists on disk
    let config = state.config();
    if !file_exists(&uri, &config).map_err(ApiError::internal)? {
        return Err(ApiError::not_found(format!(
            "Memory file not found: {identifier}"
        )));
    }

    let (frontmatter, content, _checksum) =
        read_markdown(&uri, &config).map_err(ApiError::internal)?;

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

    let records = collect_recent_activity_records(&state.config(), timespan, filter, limit)
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

    let context = ToolContext::new(state.config(), OutputFormat::Json);
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

#[derive(serde::Deserialize)]
struct AddAgentRequest {
    name: String,
    config: AgentEntry,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum AgentPayload {
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

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum McpServerPayload {
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

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum SettingsPayload {
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

async fn get_config_handler(State(state): State<ApiState>) -> ApiResult<Json<AgentsConfig>> {
    Ok(Json(state.config_manager.get_config()))
}

async fn get_config_path_handler(State(state): State<ApiState>) -> ApiResult<Json<String>> {
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
struct ReloadConfigResponse {
    config: AgentsConfig,
    reloaded: Vec<&'static str>,
    requires_restart: Vec<&'static str>,
}

async fn reload_config_handler(
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

async fn config_watch_handler(State(state): State<ApiState>) -> ApiResult<impl IntoResponse> {
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

async fn machine_id_handler() -> ApiResult<Json<String>> {
    machine_uid::get()
        .map(Json)
        .map_err(|error| ApiError::internal(format!("Failed to get machine ID: {error}")))
}

async fn discover_mcp_tools_handler(
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

async fn add_agent_handler(
    State(state): State<ApiState>,
    Json(request): Json<AddAgentRequest>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .add_agent(request.name, request.config)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

async fn update_agent_handler(
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

async fn delete_agent_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .remove_agent(&name)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

async fn add_mcp_server_handler(
    State(state): State<ApiState>,
    Json(payload): Json<McpServerPayload>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .add_mcp_server(payload.into_entry())
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

async fn update_mcp_server_handler(
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

async fn delete_mcp_server_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<AgentsConfig>> {
    let config = state
        .config_manager
        .remove_mcp_server(&name)
        .map_err(ApiError::internal)?;
    Ok(Json(config))
}

async fn update_settings_handler(
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
        .merge(crate::acp_relay::acp_routes())
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
mod tests {
    use super::{ApiState, api_routes};
    use crate::{
        Config,
        db::{self, GraphDatabase},
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use rusqlite::params;
    use tempfile::TempDir;
    use tower::ServiceExt;

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn test_state() -> (TempDir, ApiState, ConfigResetGuard) {
        let temp = TempDir::new().expect("temp dir");
        Config::override_root(temp.path());
        Config::set_database_path(temp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        GraphDatabase::initialize_database(&config).expect("initialize database");
        (temp, ApiState::new(config), ConfigResetGuard)
    }

    fn seed_graph_community_fixture(state: &ApiState) {
        let config = state.config();
        let connection = db::open_connection(&config.database_path).expect("open connection");

        for (file_path, created) in [
            ("memory/community-alpha.md", "2024-01-01T00:00:00Z"),
            ("memory/community-beta.md", "2024-01-02T00:00:00Z"),
            ("memory/community-gamma.md", "2024-01-03T00:00:00Z"),
            ("memory/community-delta.md", "2024-01-04T00:00:00Z"),
            ("memory/community-epsilon.md", "2024-01-05T00:00:00Z"),
            ("memory/community-zeta.md", "2024-01-06T00:00:00Z"),
        ] {
            connection
                .execute(
                    "INSERT INTO file_content (
                        file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed
                    ) VALUES (?1, ?2, '', '', ?3, 'indexed', '', 0, ?3, ?3)",
                    params![file_path, file_path, created],
                )
                .expect("insert file_content");
        }

        for (concept_name, occurrence_count, source_file, community_id) in [
            ("alpha", 9_i64, "memory/community-alpha.md", 1_i64),
            ("beta", 7, "memory/community-beta.md", 1),
            ("gamma", 5, "memory/community-gamma.md", 1),
            ("delta", 8, "memory/community-delta.md", 2),
            ("epsilon", 6, "memory/community-epsilon.md", 2),
            ("zeta", 4, "memory/community-zeta.md", 2),
        ] {
            connection
                .execute(
                    "INSERT INTO concepts (concept_name, first_seen, occurrence_count)
                     VALUES (?1, '2024-01-01T00:00:00Z', ?2)",
                    params![concept_name, occurrence_count],
                )
                .expect("insert concept");
            connection
                .execute(
                    "INSERT INTO concept_mentions (concept_name, source_file, mention_count)
                     VALUES (?1, ?2, 1)",
                    params![concept_name, source_file],
                )
                .expect("insert concept mention");
            connection
                .execute(
                    "INSERT INTO concept_communities (
                        concept_name, community_id, modularity, resolution, detected_at
                    ) VALUES (?1, ?2, 0.42, 1.0, '2024-01-01T00:00:00Z')",
                    params![concept_name, community_id],
                )
                .expect("insert concept community");
        }

        for (concept_a, concept_b, weight) in [
            ("alpha", "beta", 6_i64),
            ("alpha", "gamma", 4),
            ("beta", "gamma", 3),
            ("delta", "epsilon", 5),
            ("delta", "zeta", 4),
            ("epsilon", "zeta", 2),
            ("alpha", "delta", 6),
        ] {
            connection
                .execute(
                    "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                     VALUES (?1, ?2, ?3, '[]')",
                    params![concept_a, concept_b, weight],
                )
                .expect("insert concept graph edge");
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn config_path_route_returns_bare_json_string() {
        let (temp, state, _guard) = test_state();
        let expected_path = temp.path().join("maenifold.json");
        let response = api_routes(state)
            .oneshot(
                Request::builder()
                    .uri("/api/config/path")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            json,
            serde_json::Value::String(expected_path.to_string_lossy().to_string())
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn reload_config_route_reports_partial_runtime_reload() {
        let (temp, state, _guard) = test_state();
        std::fs::write(
            temp.path().join("maenifold.json"),
            r#"{
                "settings": {},
                "agents": {
                    "local": {
                        "command": "/bin/echo hi"
                    }
                }
            }"#,
        )
        .expect("write config");

        let response = api_routes(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/config/reload")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(
            json,
            serde_json::json!({
                "config": {
                    "agents": {
                        "local": {
                            "command": "/bin/echo hi"
                        }
                    },
                    "settings": serde_json::to_value(crate::acp_config::MaenifoldSettings::default())
                        .expect("default settings json")
                },
                "reloaded": ["configManagerState", "apiRuntimeSettings"],
                "requiresRestart": ["agentRuntimeBehavior"]
            })
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn api_graph_communities_endpoint_returns_seeded_community_graph() {
        let (_temp, state, _guard) = test_state();
        seed_graph_community_fixture(&state);

        let response = api_routes(state)
            .oneshot(
                Request::builder()
                    .uri("/api/graph/communities")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(
            json,
            serde_json::json!({
                "nodes": [
                    { "id": 1, "anchor": "alpha", "size": 3 },
                    { "id": 2, "anchor": "delta", "size": 3 }
                ],
                "edges": [
                    { "source": 1, "target": 2, "weight": 6 }
                ]
            })
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn api_graph_community_concepts_endpoint_returns_seeded_concept_graph() {
        let (_temp, state, _guard) = test_state();
        seed_graph_community_fixture(&state);

        let response = api_routes(state)
            .oneshot(
                Request::builder()
                    .uri("/api/graph/communities/1/concepts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(
            json,
            serde_json::json!({
                "communityId": 1,
                "anchor": "alpha",
                "nodes": [
                    { "name": "alpha", "occurrences": 9 },
                    { "name": "beta", "occurrences": 7 },
                    { "name": "gamma", "occurrences": 5 }
                ],
                "edges": [
                    { "source": "alpha", "target": "beta", "weight": 6 },
                    { "source": "alpha", "target": "gamma", "weight": 4 },
                    { "source": "beta", "target": "gamma", "weight": 3 }
                ]
            })
        );
    }
}
