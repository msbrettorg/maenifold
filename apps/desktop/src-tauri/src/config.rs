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
