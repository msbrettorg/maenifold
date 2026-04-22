use crate::api::mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT;
use agent_client_protocol as acp;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerEntry {
    #[serde(rename = "stdio")]
    Stdio {
        name: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
    },
    #[serde(rename = "http")]
    Http { name: String, url: String },
}

impl McpServerEntry {
    pub fn name(&self) -> &str {
        match self {
            McpServerEntry::Stdio { name, .. } | McpServerEntry::Http { name, .. } => name,
        }
    }

    pub fn to_acp(&self) -> acp::McpServer {
        match self {
            McpServerEntry::Stdio {
                name,
                command,
                args,
                env,
                ..
            } => acp::McpServer::Stdio(
                acp::McpServerStdio::new(name.clone(), command.clone())
                    .args(args.clone())
                    .env(
                        env.iter()
                            .map(|(name, value)| acp::EnvVariable::new(name.clone(), value.clone()))
                            .collect(),
                    ),
            ),
            McpServerEntry::Http { name, url } => {
                acp::McpServer::Http(acp::McpServerHttp::new(name.clone(), url.clone()))
            }
        }
    }

    pub fn discovery_timeout(&self) -> Duration {
        match self {
            McpServerEntry::Stdio { timeout, .. } => timeout
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_DISCOVERY_TIMEOUT),
            McpServerEntry::Http { .. } => DEFAULT_DISCOVERY_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
}

impl AgentEntry {
    pub fn command_parts(&self) -> (&str, Vec<&str>) {
        let mut parts = self.command.split_whitespace();
        let binary = parts.next().unwrap_or("");
        let args = parts.collect();
        (binary, args)
    }
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
    // T-205.716: 5000ms baseline. A higher value combined with the bounded
    // retry in `mcp::with_lock_retry` would amplify worst-case latency past
    // the 60s ACP connection timeout. With 5 retries at max 500ms backoff
    // plus a 5s busy wait per attempt, total worst-case stays comfortably
    // under the 60s ACP timeout.
    5000
}

fn default_acp_connection_timeout() -> u64 {
    60
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
    #[serde(default = "default_acp_connection_timeout")]
    pub acp_connection_timeout_seconds: u64,
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
    #[serde(default, rename = "envPassthrough")]
    pub env_passthrough: Vec<String>,
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
            acp_connection_timeout_seconds: default_acp_connection_timeout(),
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
            env_passthrough: Vec::new(),
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
    /// T-202.9a: explicit active embedding model ID (e.g. "bge-small-en-v1.5").
    /// When set, [[discover_model]] resolves `{models_dir}/{active_model}/model.onnx`
    /// before falling back to directory scanning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_model: Option<String>,
}

impl Default for VectorSettings {
    fn default() -> Self {
        Self {
            embedding_logs: false,
            vector_logs: false,
            disable_vector: false,
            active_model: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
    #[serde(default)]
    pub agents: HashMap<String, AgentEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<MaenifoldSettings>,
}

impl AgentsConfig {
    pub fn load(root: &Path) -> AgentsConfig {
        let config_path = root.join("maenifold.json");
        let Some(contents) = read_config_contents(&config_path) else {
            return Self::default();
        };

        let parsed: AgentsConfig = match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(error) => {
                warn!(
                    path = %config_path.display(),
                    error = %error,
                    "failed to parse agents config"
                );
                return Self::default();
            }
        };

        let mut agents = HashMap::new();
        for (name, entry) in parsed.agents {
            if validate_agent_entry(&name, &entry) {
                agents.insert(name, entry);
            }
        }

        Self {
            mcp_servers: parsed.mcp_servers,
            agents,
            settings: parsed.settings,
        }
    }
}

fn read_config_contents(config_path: &Path) -> Option<String> {
    match fs::read_to_string(config_path) {
        Ok(contents) => {
            info!(path = %config_path.display(), "loaded agents config file");
            Some(contents)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            warn!(
                path = %config_path.display(),
                error = %error,
                "failed to read agents config"
            );
            None
        }
    }
}

fn validate_agent_entry(name: &str, entry: &AgentEntry) -> bool {
    let (command_path, _) = entry.command_parts();
    if command_path.is_empty() {
        warn!(agent = name, "excluding agent: command must not be empty");
        return false;
    }
    let command_path = Path::new(command_path);
    if !command_path.is_absolute() {
        warn!(
            agent = name,
            command = %entry.command,
            "excluding agent: command path must be absolute"
        );
        return false;
    }

    match fs::metadata(command_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            warn!(
                agent = name,
                command = %entry.command,
                "excluding agent: command does not exist"
            );
            return false;
        }
        Err(error) => {
            warn!(
                agent = name,
                command = %entry.command,
                error = %error,
                "excluding agent: failed to inspect command"
            );
            return false;
        }
    };

    let c_path = match CString::new(command_path.as_os_str().as_encoded_bytes()) {
        Ok(path) => path,
        Err(_) => {
            warn!(
                agent = name,
                command = %entry.command,
                "excluding agent: command path contains interior NUL"
            );
            return false;
        }
    };

    // SAFETY: c_path is a valid NUL-terminated C string that lives for the duration of
    // the call, and libc::access is a read-only POSIX syscall with no memory side-effects.
    let accessible = unsafe { libc::access(c_path.as_ptr(), libc::X_OK) };
    if accessible != 0 {
        warn!(
            agent = name,
            command = %entry.command,
            "excluding agent: command is not executable by the current user"
        );
        return false;
    }

    true
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
            "acpConnectionTimeoutSeconds": 90,
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
            acp_connection_timeout_seconds: 90,
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
                active_model: None,
            },
            debug: DebugSettings {
                trace: true,
                pipeline_driver: Some("inline".to_string()),
            },
            env_passthrough: Vec::new(),
        }
    }

    #[test]
    fn settings_deserializes_from_json() {
        let settings: MaenifoldSettings =
            serde_json::from_value(sample_settings_json()).expect("settings should deserialize");

        assert_eq!(settings, expected_settings());
    }

    #[test]
    fn settings_defaults_when_absent() {
        let config: AgentsConfig = serde_json::from_value(json!({
            "agents": {},
            "mcpServers": [],
            "settings": {}
        }))
        .expect("config should deserialize");

        let settings = config.settings.expect("settings should be present");

        assert_eq!(settings.log_rotation, "daily");
        assert_eq!(settings.debounce_ms, 150);
        assert!(settings.auto_sync);
        assert!(settings.sync_logging);
        assert!(settings.session_cleanup);
        assert_eq!(settings.sqlite_busy_timeout_ms, 5000);
        assert_eq!(settings.acp_connection_timeout_seconds, 60);
        assert_eq!(settings.sync_lease_ms, 10000);
        assert_eq!(settings.watcher_buffer_size, 65536);
        assert_eq!(settings.incremental_optimize_every, 40);
        assert_eq!(settings.incremental_vacuum_minutes, 720);
        assert_eq!(settings.decay.grace_days_sequential, 7);
        assert_eq!(settings.decay.grace_days_workflows, 14);
        assert_eq!(settings.decay.grace_days_default, 28);
        assert_eq!(settings.decay.half_life_days, 30);
        assert_eq!(settings.decay.function, "power-law");
        assert_eq!(settings.community.louvain_gamma, 1.0);
        assert_eq!(settings.community.min_shared_neighbors, 3);
        assert_eq!(settings.community.min_overlap, 0.4);
        assert_eq!(settings.community.max_siblings, 10);
        assert_eq!(settings.community.debounce_ms, 2000);
        assert_eq!(settings.community.nmi_threshold, 0.8);
        assert_eq!(settings.community.recompute_file_threshold, None);
        assert!(!settings.vector.embedding_logs);
        assert!(!settings.vector.vector_logs);
        assert!(!settings.vector.disable_vector);
        assert!(!settings.debug.trace);
        assert_eq!(settings.debug.pipeline_driver, None);
        assert_eq!(settings.sync_role, None);
        assert_eq!(settings.session_abandon_minutes, None);
        assert_eq!(settings.snippet_length, None);
    }

    #[test]
    fn settings_roundtrips_through_json() {
        let settings = MaenifoldSettings::default();
        let json = serde_json::to_string(&settings).expect("settings should serialize");
        let reparsed: MaenifoldSettings =
            serde_json::from_str(&json).expect("settings should deserialize");

        assert_eq!(reparsed, settings);
    }

    #[test]
    fn agents_config_includes_settings() {
        let config: AgentsConfig = serde_json::from_value(json!({
            "mcpServers": [
                {
                    "type": "stdio",
                    "name": "local",
                    "command": "/bin/echo",
                    "args": ["hello"]
                },
                {
                    "type": "http",
                    "name": "remote",
                    "url": "http://localhost:3000/mcp"
                }
            ],
            "agents": {
                "planner": {
                    "command": "/bin/echo --plan",
                    "env": {
                        "MODE": "test"
                    },
                    "mcpServers": [
                        {
                            "type": "stdio",
                            "name": "local",
                            "command": "/bin/echo",
                            "args": ["planner"]
                        }
                    ]
                }
            },
            "settings": sample_settings_json()
        }))
        .expect("config should deserialize");

        let settings = config.settings.expect("settings should be present");
        assert_eq!(settings, expected_settings());
        assert_eq!(config.mcp_servers.len(), 2);
        assert_eq!(config.mcp_servers[0].name(), "local");
        assert_eq!(config.mcp_servers[1].name(), "remote");

        let agent = config
            .agents
            .get("planner")
            .expect("planner agent should exist");
        assert_eq!(agent.command, "/bin/echo --plan");
        assert_eq!(agent.env.get("MODE").map(String::as_str), Some("test"));
        assert_eq!(agent.mcp_servers.len(), 1);
        assert_eq!(agent.mcp_servers[0].name(), "local");
    }

    #[test]
    fn test_real_maenifold_json_core() {
        let home = std::env::var("HOME").expect("HOME should be set");
        let root = Path::new(&home).join("maenifold");
        let config = AgentsConfig::load(&root);

        assert!(
            !config.mcp_servers.is_empty(),
            "expected at least one top-level mcp server from {}",
            root.join("maenifold.json").display()
        );
    }

    #[test]
    fn agents_config_settings_optional() {
        let config: AgentsConfig = serde_json::from_value(json!({
            "agents": {
                "planner": {
                    "command": "/bin/echo --plan"
                }
            }
        }))
        .expect("config should deserialize");

        assert_eq!(config.settings, None);
        let agent = config
            .agents
            .get("planner")
            .expect("planner agent should exist");
        assert_eq!(agent.command, "/bin/echo --plan");
        assert!(agent.env.is_empty());
        assert!(agent.mcp_servers.is_empty());
    }
}
