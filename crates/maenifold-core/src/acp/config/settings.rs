// T-01.16: settings struct definitions extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use serde::{Deserialize, Serialize};

use crate::teams::MessageHandlerConfig;

pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_log_rotation() -> String {
    "daily".to_string()
}

pub(super) fn default_debounce_ms() -> u64 {
    150
}

pub(super) fn default_sqlite_busy_timeout() -> u64 {
    // T-205.716: 5000ms baseline. A higher value combined with the bounded
    // retry in `mcp::with_lock_retry` would amplify worst-case latency past
    // the 60s ACP connection timeout. With 5 retries at max 500ms backoff
    // plus a 5s busy wait per attempt, total worst-case stays comfortably
    // under the 60s ACP timeout.
    5000
}

pub(super) fn default_acp_connection_timeout() -> u64 {
    60
}

pub(super) fn default_sync_lease_ms() -> u64 {
    10000
}

pub(super) fn default_watcher_buffer() -> u64 {
    65536
}

pub(super) fn default_optimize_every() -> u32 {
    40
}

pub(super) fn default_vacuum_minutes() -> u32 {
    720
}

pub(super) fn default_7() -> i64 {
    7
}

pub(super) fn default_14() -> i64 {
    14
}

pub(super) fn default_28() -> i64 {
    28
}

pub(super) fn default_30() -> i64 {
    30
}

pub(super) fn default_decay_function() -> String {
    "power-law".to_string()
}

pub(super) fn default_louvain_gamma() -> f64 {
    1.0
}

pub(super) fn default_min_shared() -> u64 {
    3
}

pub(super) fn default_min_overlap() -> f64 {
    0.4
}

pub(super) fn default_max_siblings() -> u64 {
    10
}

pub(super) fn default_community_debounce() -> u64 {
    2000
}

pub(super) fn default_nmi_threshold() -> f64 {
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
    #[serde(default)]
    pub teams_monitor: TeamsMonitorConfig,
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
            teams_monitor: TeamsMonitorConfig::default(),
            env_passthrough: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamsMonitorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub handler: MessageHandlerConfig,
    #[serde(default)]
    pub dispatch_agent: Option<String>,
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
