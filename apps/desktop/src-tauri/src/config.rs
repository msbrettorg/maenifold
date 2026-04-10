use indexmap::IndexMap;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerEntry {
    #[serde(rename = "stdio")]
    Stdio {
        name: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    #[serde(rename = "http")]
    Http { name: String, url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub command: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
}

fn default_true() -> bool {
    true
}

fn default_log_rotation() -> String {
    "daily".to_string()
}

fn default_debounce_ms() -> u64 {
    150
}

fn default_sqlite_busy_timeout() -> u64 {
    5000
}

fn default_sync_lease_ms() -> u64 {
    10000
}

fn default_watcher_buffer() -> u64 {
    65536
}

fn default_optimize_every() -> u32 {
    40
}

fn default_vacuum_minutes() -> u32 {
    720
}

fn default_7() -> i64 {
    7
}

fn default_14() -> i64 {
    14
}

fn default_28() -> i64 {
    28
}

fn default_30() -> i64 {
    30
}

fn default_decay_function() -> String {
    "power-law".to_string()
}

fn default_louvain_gamma() -> f64 {
    1.0
}

fn default_min_shared() -> u64 {
    3
}

fn default_min_overlap() -> f64 {
    0.4
}

fn default_max_siblings() -> u64 {
    10
}

fn default_community_debounce() -> u64 {
    2000
}

fn default_nmi_threshold() -> f64 {
    0.8
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaenifoldSettings {
    #[serde(default = "default_log_rotation")]
    pub log_rotation: String,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_true")]
    pub auto_sync: bool,
    #[serde(default = "default_true")]
    pub sync_logging: bool,
    #[serde(default = "default_true")]
    pub session_cleanup: bool,
    #[serde(default = "default_sqlite_busy_timeout")]
    pub sqlite_busy_timeout_ms: u64,
    #[serde(default = "default_sync_lease_ms")]
    pub sync_lease_ms: u64,
    #[serde(default = "default_watcher_buffer")]
    pub watcher_buffer_size: u64,
    #[serde(default = "default_optimize_every")]
    pub incremental_optimize_every: u32,
    #[serde(default = "default_vacuum_minutes")]
    pub incremental_vacuum_minutes: u32,
    #[serde(default)]
    pub session_abandon_minutes: Option<u64>,
    #[serde(default)]
    pub snippet_length: Option<u64>,
    #[serde(default)]
    pub sync_role: Option<String>,
    #[serde(default)]
    pub decay: DecaySettings,
    #[serde(default)]
    pub community: CommunitySettings,
    #[serde(default)]
    pub vector: VectorSettings,
    #[serde(default)]
    pub debug: DebugSettings,
}

impl Default for MaenifoldSettings {
    fn default() -> Self {
        Self {
            log_rotation: default_log_rotation(),
            debounce_ms: default_debounce_ms(),
            auto_sync: default_true(),
            sync_logging: default_true(),
            session_cleanup: default_true(),
            sqlite_busy_timeout_ms: default_sqlite_busy_timeout(),
            sync_lease_ms: default_sync_lease_ms(),
            watcher_buffer_size: default_watcher_buffer(),
            incremental_optimize_every: default_optimize_every(),
            incremental_vacuum_minutes: default_vacuum_minutes(),
            session_abandon_minutes: None,
            snippet_length: None,
            sync_role: None,
            decay: DecaySettings::default(),
            community: CommunitySettings::default(),
            vector: VectorSettings::default(),
            debug: DebugSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecaySettings {
    #[serde(default = "default_7")]
    pub grace_days_sequential: i64,
    #[serde(default = "default_14")]
    pub grace_days_workflows: i64,
    #[serde(default = "default_28")]
    pub grace_days_default: i64,
    #[serde(default = "default_30")]
    pub half_life_days: i64,
    #[serde(default = "default_decay_function")]
    pub function: String,
}

impl Default for DecaySettings {
    fn default() -> Self {
        Self {
            grace_days_sequential: default_7(),
            grace_days_workflows: default_14(),
            grace_days_default: default_28(),
            half_life_days: default_30(),
            function: default_decay_function(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommunitySettings {
    #[serde(default = "default_louvain_gamma")]
    pub louvain_gamma: f64,
    #[serde(default = "default_min_shared")]
    pub min_shared_neighbors: u64,
    #[serde(default = "default_min_overlap")]
    pub min_overlap: f64,
    #[serde(default = "default_max_siblings")]
    pub max_siblings: u64,
    #[serde(default = "default_community_debounce")]
    pub debounce_ms: u64,
    #[serde(default = "default_nmi_threshold")]
    pub nmi_threshold: f64,
    #[serde(default)]
    pub recompute_file_threshold: Option<u64>,
}

impl Default for CommunitySettings {
    fn default() -> Self {
        Self {
            louvain_gamma: default_louvain_gamma(),
            min_shared_neighbors: default_min_shared(),
            min_overlap: default_min_overlap(),
            max_siblings: default_max_siblings(),
            debounce_ms: default_community_debounce(),
            nmi_threshold: default_nmi_threshold(),
            recompute_file_threshold: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VectorSettings {
    #[serde(default)]
    pub embedding_logs: bool,
    #[serde(default)]
    pub vector_logs: bool,
    #[serde(default)]
    pub disable_vector: bool,
}

impl Default for VectorSettings {
    fn default() -> Self {
        Self {
            embedding_logs: false,
            vector_logs: false,
            disable_vector: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DebugSettings {
    #[serde(default)]
    pub trace: bool,
    #[serde(default)]
    pub pipeline_driver: Option<String>,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            trace: false,
            pipeline_driver: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
    #[serde(default)]
    pub agents: IndexMap<String, AgentConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<MaenifoldSettings>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            agents: IndexMap::new(),
            settings: None,
        }
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
    #[allow(dead_code)]
    watcher: Option<RecommendedWatcher>,
}

impl ConfigManager {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let config_path = get_config_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Load initial config or create default
        let config = if config_path.exists() {
            load_config(&config_path)?
        } else {
            let default_config = AgentsConfig::default();
            save_config(&config_path, &default_config)?;
            default_config
        };

        let config = Arc::new(RwLock::new(config));
        let config_clone = Arc::clone(&config);
        let config_path_clone = config_path.clone();
        let app_handle = app.clone();

        // Set up file watcher
        let watcher = setup_watcher(config_clone, config_path_clone, app_handle)?;

        Ok(Self {
            config,
            config_path,
            watcher: Some(watcher),
        })
    }

    pub fn get_config(&self) -> AgentsConfig {
        self.config.read().clone()
    }

    pub fn reload(&self) -> Result<AgentsConfig, String> {
        let new_config = load_config(&self.config_path)?;
        *self.config.write() = new_config.clone();
        Ok(new_config)
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    pub fn save(&self) -> Result<(), String> {
        let config = self.config.read();
        save_config(&self.config_path, &config)
    }

    pub fn add_agent(&self, name: String, config: AgentConfig) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.insert(name, config);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn remove_agent(&self, name: &str) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.shift_remove(name);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn update_agent(&self, name: String, config: AgentConfig) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            if agents_config.agents.contains_key(&name) {
                agents_config.agents.insert(name, config);
            } else {
                return Err(format!("Agent '{}' not found", name));
            }
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn add_mcp_server(&self, entry: McpServerEntry) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.mcp_servers.push(entry);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn remove_mcp_server(&self, name: &str) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.mcp_servers.retain(|s| match s {
                McpServerEntry::Stdio { name: n, .. } | McpServerEntry::Http { name: n, .. } => {
                    n != name
                }
            });
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn update_mcp_server(&self, name: &str, entry: McpServerEntry) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            if let Some(existing) = config.mcp_servers.iter_mut().find(|s| match s {
                McpServerEntry::Stdio { name: n, .. } | McpServerEntry::Http { name: n, .. } => {
                    n == name
                }
            }) {
                *existing = entry;
            } else {
                return Err(format!("MCP server '{}' not found", name));
            }
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn update_settings(&self, settings: MaenifoldSettings) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.settings = Some(settings);
        }
        self.save()?;
        Ok(self.get_config())
    }
}

fn get_config_path() -> Result<PathBuf, String> {
    // Prefer $MAENIFOLD_ROOT/maenifold.json so both maenifold and pipeline-ui share one config
    if let Ok(root) = std::env::var("MAENIFOLD_ROOT") {
        let path = PathBuf::from(root).join("maenifold.json");
        if path.exists() || path.parent().is_some_and(|p| p.exists()) {
            return Ok(path);
        }
    }

    // Default to ~/maenifold/maenifold.json (same default as maenifold-rs Config::current())
    dirs::home_dir()
        .map(|p| p.join("maenifold").join("maenifold.json"))
        .ok_or_else(|| "Could not find home directory".to_string())
}

fn load_config(path: &PathBuf) -> Result<AgentsConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_config(path: &PathBuf, config: &AgentsConfig) -> Result<(), String> {
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn setup_watcher(
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
    app_handle: AppHandle,
) -> Result<RecommendedWatcher, String> {
    let config_path_for_watcher = config_path.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        if event.paths.iter().any(|p| p == &config_path_for_watcher) {
                            if let Ok(new_config) = load_config(&config_path_for_watcher) {
                                *config.write() = new_config.clone();
                                let _ = app_handle.emit("config-changed", new_config);
                            }
                        }
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;

    // Watch the config directory
    if let Some(parent) = config_path.parent() {
        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }

    Ok(watcher)
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    use serde_json::json;

    fn sample_settings_json() -> serde_json::Value {
        json!({
            "logRotation": "hourly",
            "debounceMs": 250,
            "autoSync": false,
            "syncLogging": false,
            "sessionCleanup": false,
            "sqliteBusyTimeoutMs": 7000,
            "syncLeaseMs": 15000,
            "watcherBufferSize": 131072,
            "incrementalOptimizeEvery": 55,
            "incrementalVacuumMinutes": 360,
            "sessionAbandonMinutes": 45,
            "snippetLength": 512,
            "syncRole": "leader",
            "decay": {
                "graceDaysSequential": 9,
                "graceDaysWorkflows": 18,
                "graceDaysDefault": 35,
                "halfLifeDays": 45,
                "function": "exponential"
            },
            "community": {
                "louvainGamma": 1.25,
                "minSharedNeighbors": 5,
                "minOverlap": 0.55,
                "maxSiblings": 12,
                "debounceMs": 3500,
                "nmiThreshold": 0.9,
                "recomputeFileThreshold": 100
            },
            "vector": {
                "embeddingLogs": true,
                "vectorLogs": true,
                "disableVector": true
            },
            "debug": {
                "trace": true,
                "pipelineDriver": "inline"
            }
        })
    }

    fn expected_settings() -> MaenifoldSettings {
        MaenifoldSettings {
            log_rotation: "hourly".to_string(),
            debounce_ms: 250,
            auto_sync: false,
            sync_logging: false,
            session_cleanup: false,
            sqlite_busy_timeout_ms: 7000,
            sync_lease_ms: 15000,
            watcher_buffer_size: 131072,
            incremental_optimize_every: 55,
            incremental_vacuum_minutes: 360,
            session_abandon_minutes: Some(45),
            snippet_length: Some(512),
            sync_role: Some("leader".to_string()),
            decay: DecaySettings {
                grace_days_sequential: 9,
                grace_days_workflows: 18,
                grace_days_default: 35,
                half_life_days: 45,
                function: "exponential".to_string(),
            },
            community: CommunitySettings {
                louvain_gamma: 1.25,
                min_shared_neighbors: 5,
                min_overlap: 0.55,
                max_siblings: 12,
                debounce_ms: 3500,
                nmi_threshold: 0.9,
                recompute_file_threshold: Some(100),
            },
            vector: VectorSettings {
                embedding_logs: true,
                vector_logs: true,
                disable_vector: true,
            },
            debug: DebugSettings {
                trace: true,
                pipeline_driver: Some("inline".to_string()),
            },
        }
    }

    #[test]
    fn pipeline_settings_deserializes() {
        let config: AgentsConfig = serde_json::from_value(json!({
            "mcpServers": [
                {
                    "type": "stdio",
                    "name": "local",
                    "command": "/bin/echo",
                    "args": ["hello"]
                }
            ],
            "agents": {
                "planner": {
                    "command": "/bin/echo --plan"
                }
            },
            "settings": sample_settings_json()
        }))
        .expect("config should deserialize");

        let settings = config.settings.expect("settings should be present");
        assert_eq!(settings, expected_settings());
        assert_eq!(config.mcp_servers.len(), 1);
        match &config.mcp_servers[0] {
            McpServerEntry::Stdio { name, .. } => assert_eq!(name, "local"),
            other => panic!("expected stdio server, got {other:?}"),
        }
        assert!(config.agents.contains_key("planner"));
    }

    #[test]
    fn pipeline_settings_roundtrip() {
        let settings = MaenifoldSettings::default();
        let json = serde_json::to_string(&settings).expect("settings should serialize");
        let reparsed: MaenifoldSettings =
            serde_json::from_str(&json).expect("settings should deserialize");

        assert_eq!(reparsed, settings);
    }

    #[test]
    fn pipeline_settings_absent_is_none() {
        let config: AgentsConfig = serde_json::from_value(json!({
            "agents": {
                "planner": {
                    "command": "/bin/echo --plan"
                }
            }
        }))
        .expect("config should deserialize");

        assert_eq!(config.settings, None);
        assert!(config.agents.contains_key("planner"));
    }

    #[test]
    fn test_real_maenifold_json() {
        let config_path = dirs::home_dir()
            .expect("home directory should exist")
            .join("maenifold")
            .join("maenifold.json");
        let content = fs::read_to_string(&config_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", config_path.display()));
        let raw_config: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|error| panic!("failed to parse raw {}: {error}", config_path.display()));
        let config: AgentsConfig = serde_json::from_str(&content)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", config_path.display()));
        let raw_mcp_server_count = raw_config
            .get("mcpServers")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .expect("mcpServers should be present and be an array");

        assert_eq!(config.mcp_servers.len(), raw_mcp_server_count);
    }
}

#[cfg(test)]
mod mcp_integration_tests {
    use super::*;
    use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
    use tokio::process::{Child, ChildStdin, ChildStdout, Command};
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
    const PROCESS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

    #[derive(Debug)]
    struct ServerCheckResult {
        tool_count: usize,
    }

    fn build_http_headers(session_id: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        match session_id {
            Some(session_id) => {
                headers.insert(
                    "Mcp-Session-Id",
                    HeaderValue::from_str(session_id).expect("session id should be a valid header"),
                );
            }
            None => {
                headers.insert(
                    "Mcp-Protocol-Version",
                    HeaderValue::from_static(MCP_PROTOCOL_VERSION),
                );
            }
        }

        headers
    }

    async fn send_http_request(
        client: &reqwest::Client,
        url: &str,
        session_id: Option<&str>,
        body: &Value,
    ) -> Result<reqwest::Response, String> {
        client
            .post(url)
            .headers(build_http_headers(session_id))
            .json(body)
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))
    }

    async fn parse_http_jsonrpc_response(
        response: reqwest::Response,
        expected_id: u64,
    ) -> Result<Value, String> {
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let payload = if content_type.contains("text/event-stream") {
            let body = response
                .text()
                .await
                .map_err(|error| format!("failed to read SSE body: {error}"))?;
            parse_sse_jsonrpc_response(&body, expected_id)?
        } else {
            response
                .json::<Value>()
                .await
                .map_err(|error| format!("failed to parse JSON body: {error}"))?
        };

        match payload.get("error") {
            Some(error) if !error.is_null() => Err(format!("JSON-RPC error: {error}")),
            _ => Ok(payload),
        }
    }

    fn parse_sse_jsonrpc_response(body: &str, expected_id: u64) -> Result<Value, String> {
        for block in body.split("\n\n").flat_map(|chunk| chunk.split("\r\n\r\n")) {
            let data_lines = block
                .lines()
                .map(str::trim)
                .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();

            if data_lines.is_empty() {
                continue;
            }

            let candidate = data_lines.join("\n");
            let parsed = match serde_json::from_str::<Value>(&candidate) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            };

            let id_matches = parsed
                .get("id")
                .and_then(Value::as_u64)
                .is_some_and(|id| id == expected_id);
            if id_matches {
                return Ok(parsed);
            }
        }

        Err(format!(
            "no valid JSON-RPC response found in SSE stream for request id {expected_id}"
        ))
    }

    async fn send_http_initialized_notification(
        client: &reqwest::Client,
        url: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let response = client
            .post(url)
            .headers(build_http_headers(session_id))
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .send()
            .await
            .map_err(|error| format!("initialized notification failed: {error}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "initialized notification failed with HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    async fn delete_http_session(
        client: &reqwest::Client,
        url: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let Some(session_id) = session_id else {
            return Ok(());
        };

        let response = client
            .delete(url)
            .header(ACCEPT, "application/json, text/event-stream")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .map_err(|error| format!("session delete failed: {error}"))?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(format!("session delete failed with HTTP {status}"));
        }

        Ok(())
    }

    async fn check_http_server(name: &str, url: &str) -> Result<ServerCheckResult, String> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;

        let initialize_response = send_http_request(
            &client,
            url,
            None,
            &json!({
                "jsonrpc": "2.0",
                "id": 1u64,
                "method": "initialize",
                "params": {
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "maenifold-desktop-test",
                        "version": "0.1.0"
                    }
                }
            }),
        )
        .await?;

        let status = initialize_response.status();
        let session_id = initialize_response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        if !status.is_success() {
            return Err(format!("initialize failed with HTTP {status}"));
        }

        let initialize_payload = parse_http_jsonrpc_response(initialize_response, 1).await?;
        let server_info = initialize_payload
            .get("result")
            .and_then(|result| result.get("serverInfo"))
            .ok_or_else(|| "initialize response missing result.serverInfo".to_string())?;
        let server_name = server_info
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");

        send_http_initialized_notification(&client, url, session_id.as_deref()).await?;

        let tools_response = send_http_request(
            &client,
            url,
            session_id.as_deref(),
            &json!({
                "jsonrpc": "2.0",
                "id": 2u64,
                "method": "tools/list"
            }),
        )
        .await?;
        let tools_status = tools_response.status();
        if !tools_status.is_success() {
            let _ = delete_http_session(&client, url, session_id.as_deref()).await;
            return Err(format!("tools/list failed with HTTP {tools_status}"));
        }

        let tools_payload = parse_http_jsonrpc_response(tools_response, 2).await?;
        let tools = tools_payload
            .get("result")
            .and_then(|result| result.get("tools"))
            .and_then(Value::as_array)
            .ok_or_else(|| "tools/list response missing result.tools".to_string())?;
        if tools.is_empty() {
            let _ = delete_http_session(&client, url, session_id.as_deref()).await;
            return Err(format!(
                "tools/list returned zero tools for server '{name}' (serverInfo.name='{server_name}')"
            ));
        }

        delete_http_session(&client, url, session_id.as_deref()).await?;
        Ok(ServerCheckResult {
            tool_count: tools.len(),
        })
    }

    async fn send_stdio_message(stdin: &mut ChildStdin, message: &Value) -> Result<(), String> {
        let mut body =
            serde_json::to_vec(message).map_err(|error| format!("failed to serialize message: {error}"))?;
        body.push(b'\n');
        stdin
            .write_all(&body)
            .await
            .map_err(|error| format!("failed to write message body: {error}"))?;
        stdin
            .flush()
            .await
            .map_err(|error| format!("failed to flush stdin: {error}"))?;
        Ok(())
    }

    async fn read_stdio_message<R>(reader: &mut BufReader<R>) -> Result<Value, String>
    where
        R: AsyncRead + Unpin,
    {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .await
            .map_err(|error| format!("failed to read MCP line: {error}"))?;
        if bytes == 0 {
            return Err("unexpected EOF while reading MCP response".to_string());
        }

        serde_json::from_str(line.trim())
            .map_err(|error| format!("failed to parse MCP line as JSON: {error}; line={}", line.trim()))
    }

    async fn read_stdio_response_for_id<R>(
        reader: &mut BufReader<R>,
        expected_id: u64,
    ) -> Result<Value, String>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            let message = read_stdio_message(reader).await?;
            let id_matches = message
                .get("id")
                .and_then(Value::as_u64)
                .is_some_and(|id| id == expected_id);
            if id_matches {
                if let Some(error) = message.get("error") {
                    if !error.is_null() {
                        return Err(format!("JSON-RPC error: {error}"));
                    }
                }
                return Ok(message);
            }
        }
    }

    async fn collect_stderr(
        stderr: tokio::process::ChildStderr,
        sink: Arc<Mutex<String>>,
    ) -> Result<(), String> {
        let mut reader = BufReader::new(stderr);
        loop {
            let mut line = String::new();
            let bytes = reader
                .read_line(&mut line)
                .await
                .map_err(|error| format!("failed to read stderr: {error}"))?;
            if bytes == 0 {
                break;
            }
            sink.lock().await.push_str(&line);
        }
        Ok(())
    }

    async fn shutdown_child(
        child: &mut Child,
        stderr_task: tokio::task::JoinHandle<Result<(), String>>,
        stderr_text: Arc<Mutex<String>>,
    ) -> Result<String, String> {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                child
                    .kill()
                    .await
                    .map_err(|error| format!("failed to kill child process: {error}"))?;
                timeout(PROCESS_SHUTDOWN_TIMEOUT, child.wait())
                    .await
                    .map_err(|_| "timed out waiting for child process to exit".to_string())?
                    .map_err(|error| format!("failed to wait for child process: {error}"))?;
            }
            Err(error) => return Err(format!("failed to inspect child process: {error}")),
        }

        match timeout(Duration::from_secs(1), stderr_task).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => return Err(error),
            Ok(Err(error)) => return Err(format!("stderr task join failed: {error}")),
            Err(_) => {}
        }

        Ok(stderr_text.lock().await.clone())
    }

    async fn check_stdio_server(
        name: &str,
        command: &str,
        args: &[String],
    ) -> Result<ServerCheckResult, String> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn '{command} {}': {error}", args.join(" ")))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "child stdin was not piped".to_string())?;
        let stdout: ChildStdout = child
            .stdout
            .take()
            .ok_or_else(|| "child stdout was not piped".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "child stderr was not piped".to_string())?;

        let stderr_text = Arc::new(Mutex::new(String::new()));
        let stderr_task = tokio::spawn(collect_stderr(stderr, Arc::clone(&stderr_text)));
        let mut stdout = BufReader::new(stdout);

        let run = async {
            send_stdio_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 1u64,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": MCP_PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": {
                            "name": "maenifold-desktop-test",
                            "version": "0.1.0"
                        }
                    }
                }),
            )
            .await?;

            let initialize_payload = read_stdio_response_for_id(&mut stdout, 1).await?;
            initialize_payload
                .get("result")
                .and_then(|result| result.get("serverInfo"))
                .ok_or_else(|| "initialize response missing result.serverInfo".to_string())?;

            send_stdio_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }),
            )
            .await?;

            send_stdio_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 2u64,
                    "method": "tools/list"
                }),
            )
            .await?;

            let tools_payload = read_stdio_response_for_id(&mut stdout, 2).await?;
            let tools = tools_payload
                .get("result")
                .and_then(|result| result.get("tools"))
                .and_then(Value::as_array)
                .ok_or_else(|| "tools/list response missing result.tools".to_string())?;
            if tools.is_empty() {
                return Err(format!("tools/list returned zero tools for server '{name}'"));
            }

            Ok(ServerCheckResult {
                tool_count: tools.len(),
            })
        };

        let result = timeout(REQUEST_TIMEOUT, run)
            .await
            .map_err(|_| format!("timed out waiting for stdio server '{name}'"))?;

        let stderr_output = shutdown_child(&mut child, stderr_task, stderr_text).await?;
        result.map_err(|error| {
            if stderr_output.trim().is_empty() {
                error
            } else {
                format!("{error}\nstderr:\n{}", stderr_output.trim())
            }
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_all_configured_mcp_servers() {
        let config_path = get_config_path().expect("config path should resolve");
        let config = load_config(&config_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", config_path.display()));

        assert!(
            !config.mcp_servers.is_empty(),
            "expected at least one configured MCP server in {}",
            config_path.display()
        );

        let mut failures = Vec::new();

        for server in &config.mcp_servers {
            match server {
                McpServerEntry::Http { name, url } => match check_http_server(name, url).await {
                    Ok(result) => {
                        println!("OK  name={name} type=http tools={}", result.tool_count);
                    }
                    Err(error) => {
                        println!("ERR name={name} type=http error={error}");
                        failures.push(format!("{name} (http): {error}"));
                    }
                },
                McpServerEntry::Stdio {
                    name,
                    command,
                    args,
                } => {
                    println!("RUN name={name} type=stdio command={} args={}", command, args.join(" "));
                    match check_stdio_server(name, command, args).await {
                    Ok(result) => {
                        println!("OK  name={name} type=stdio tools={}", result.tool_count);
                    }
                    Err(error) => {
                        println!("ERR name={name} type=stdio error={error}");
                        failures.push(format!("{name} (stdio): {error}"));
                    }
                }
                }
            }
        }

        assert!(
            failures.is_empty(),
            "configured MCP server checks failed:\n{}",
            failures.join("\n")
        );
    }
}
