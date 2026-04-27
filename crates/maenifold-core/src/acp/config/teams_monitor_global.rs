// T-238.59: Global + per-workspace Teams monitor config.
// Mirrors `agency-cowork/skills/teams/scripts/monitor/config.py`.
// This module only defines structs, defaults, normalization, and
// load/save I/O. Migration & supervisor wiring live in later sub-tasks.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MaenifoldError, Result};

// ── Defaults (mirror Python `DispatchConfig.__init__`) ─────────────────────
fn d_command() -> String {
    "agency copilot".to_string()
}
fn d_timeout_minutes() -> u64 {
    30
}
fn d_send_receipt() -> bool {
    true
}
fn d_response_conversation() -> String {
    "48:notes".to_string()
}
fn d_use_persistent_pty() -> bool {
    true
}
fn d_pty_queue_max() -> u32 {
    5
}
fn d_pty_idle_timeout_minutes() -> u64 {
    60
}
fn d_pty_max_sessions() -> u32 {
    5
}
fn d_pty_stale_dispatch_timeout_minutes() -> u64 {
    5
}
fn d_pty_warmup_conversations() -> Vec<String> {
    vec!["48:notes".to_string()]
}

fn d_keyword() -> String {
    "@agent".to_string()
}
fn d_reply_prefix() -> String {
    "Agency Cowork: ".to_string()
}

fn d_sender_mri() -> String {
    "8:orgid:00000000-0000-0000-0000-000000000000".to_string()
}
fn d_sender_display_name() -> String {
    "Your Name".to_string()
}
fn d_sender_upn() -> String {
    "user@contoso.com".to_string()
}

fn d_trouter_gateway() -> String {
    "go-msit.trouter.teams.microsoft.com".to_string()
}
fn d_registrar_url() -> String {
    "https://teams.cloud.microsoft/registrar/prod/V2/registrations".to_string()
}
fn d_app_id() -> String {
    "AgencyCoworkMonitor".to_string()
}
fn d_reconnect_delay_seconds() -> u64 {
    5
}
fn d_max_reconnect_delay_seconds() -> u64 {
    300
}
fn d_token_refresh_minutes() -> u64 {
    50
}
fn d_heartbeat_interval_seconds() -> u64 {
    30
}

fn d_conversation_type() -> String {
    "Unknown".to_string()
}
fn d_added_now() -> String {
    Utc::now().to_rfc3339()
}
fn d_default_monitored_conversations() -> Vec<MonitoredConversation> {
    vec![MonitoredConversation {
        id: "48:notes".to_string(),
        name: "Self (Notes)".to_string(),
        type_: "Self".to_string(),
        added: d_added_now(),
    }]
}
fn d_workspace_dispatch() -> WorkspaceDispatchConfig {
    let mut dispatch = WorkspaceDispatchConfig::default();
    dispatch.persistent_session_id = Uuid::new_v4().to_string();
    dispatch
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceDispatchConfig {
    #[serde(default = "d_command")]
    pub command: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default = "d_timeout_minutes")]
    pub timeout_minutes: u64,
    #[serde(default = "d_send_receipt")]
    pub send_receipt: bool,
    #[serde(default)]
    pub send_result_summary: bool,
    #[serde(default = "d_response_conversation")]
    pub response_conversation: String,
    #[serde(default)]
    pub persistent_session_id: String,
    #[serde(default = "d_use_persistent_pty")]
    pub use_persistent_pty: bool,
    #[serde(default = "d_pty_warmup_conversations")]
    pub pty_warmup_conversations: Vec<String>,
    #[serde(default = "d_pty_queue_max")]
    pub pty_queue_max: u32,
    #[serde(default = "d_pty_idle_timeout_minutes")]
    pub pty_idle_timeout_minutes: u64,
    #[serde(default = "d_pty_max_sessions")]
    pub pty_max_sessions: u32,
    #[serde(default = "d_pty_stale_dispatch_timeout_minutes")]
    pub pty_stale_dispatch_timeout_minutes: u64,
}

impl Default for WorkspaceDispatchConfig {
    fn default() -> Self {
        Self {
            command: d_command(),
            working_directory: String::new(),
            timeout_minutes: d_timeout_minutes(),
            send_receipt: d_send_receipt(),
            send_result_summary: false,
            response_conversation: d_response_conversation(),
            persistent_session_id: String::new(),
            use_persistent_pty: d_use_persistent_pty(),
            pty_warmup_conversations: vec!["48:notes".to_string()],
            pty_queue_max: d_pty_queue_max(),
            pty_idle_timeout_minutes: d_pty_idle_timeout_minutes(),
            pty_max_sessions: d_pty_max_sessions(),
            pty_stale_dispatch_timeout_minutes: d_pty_stale_dispatch_timeout_minutes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MonitoredConversation {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "d_conversation_type", rename = "type")]
    pub type_: String,
    #[serde(default = "d_added_now")]
    pub added: String,
}

impl Default for MonitoredConversation {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            type_: d_conversation_type(),
            added: d_added_now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizedSender {
    #[serde(default = "d_sender_mri")]
    pub mri: String,
    #[serde(default = "d_sender_display_name", rename = "displayName")]
    pub display_name: String,
    #[serde(default = "d_sender_upn")]
    pub upn: String,
}

impl Default for AuthorizedSender {
    fn default() -> Self {
        Self {
            mri: d_sender_mri(),
            display_name: d_sender_display_name(),
            upn: d_sender_upn(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TeamsMonitorGlobalConnectionConfig {
    #[serde(default = "d_trouter_gateway")]
    pub trouter_gateway: String,
    #[serde(default = "d_registrar_url")]
    pub registrar_url: String,
    #[serde(default = "d_app_id")]
    pub app_id: String,
    #[serde(default = "d_reconnect_delay_seconds")]
    pub reconnect_delay_seconds: u64,
    #[serde(default = "d_max_reconnect_delay_seconds")]
    pub max_reconnect_delay_seconds: u64,
    #[serde(default = "d_token_refresh_minutes")]
    pub token_refresh_minutes: u64,
    #[serde(default = "d_heartbeat_interval_seconds")]
    pub heartbeat_interval_seconds: u64,
    #[serde(default)]
    pub chatsvc_region: String,
}

impl Default for TeamsMonitorGlobalConnectionConfig {
    fn default() -> Self {
        Self {
            trouter_gateway: d_trouter_gateway(),
            registrar_url: d_registrar_url(),
            app_id: d_app_id(),
            reconnect_delay_seconds: d_reconnect_delay_seconds(),
            max_reconnect_delay_seconds: d_max_reconnect_delay_seconds(),
            token_refresh_minutes: d_token_refresh_minutes(),
            heartbeat_interval_seconds: d_heartbeat_interval_seconds(),
            chatsvc_region: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceTeamsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "d_keyword")]
    pub keyword: String,
    #[serde(default = "d_reply_prefix")]
    pub reply_prefix: String,
    #[serde(default = "d_default_monitored_conversations")]
    pub monitored_conversations: Vec<MonitoredConversation>,
    #[serde(default = "d_workspace_dispatch")]
    pub dispatch: WorkspaceDispatchConfig,
}

impl Default for WorkspaceTeamsConfig {
    fn default() -> Self {
        let mut dispatch = WorkspaceDispatchConfig::default();
        dispatch.persistent_session_id = Uuid::new_v4().to_string();
        Self {
            enabled: false,
            keyword: d_keyword(),
            reply_prefix: d_reply_prefix(),
            monitored_conversations: d_default_monitored_conversations(),
            dispatch,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TeamsMonitorGlobalConfig {
    #[serde(default)]
    pub identity: AuthorizedSender,
    #[serde(default)]
    pub connection: TeamsMonitorGlobalConnectionConfig,
    #[serde(default)]
    pub workspaces: HashMap<String, WorkspaceTeamsConfig>,
}

/// Normalize a workspace path to a stable map key. Mirrors Python
/// `_normalise_workspace_key`: canonicalize when possible (fall back to the
/// path as given), and lowercase on Windows.
pub fn normalize_workspace_key(path: &Path) -> String {
    let resolved: PathBuf = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = resolved.to_string_lossy().to_string();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

impl TeamsMonitorGlobalConfig {
    /// Load the global config from `path`. Returns `Default` if the file
    /// does not exist; surfaces parse errors.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path).map_err(|e| {
            MaenifoldError::Message(format!(
                "failed to read teams-monitor global config '{}': {e}",
                path.display()
            ))
        })?;
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&contents).map_err(|e| {
            MaenifoldError::Message(format!(
                "failed to parse teams-monitor global config '{}': {e}",
                path.display()
            ))
        })
    }

    /// Atomic write via `*.tmp` + rename.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_vec_pretty(self)?;
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, path).map_err(|e| {
            MaenifoldError::Message(format!("failed to replace '{}': {e}", path.display()))
        })?;
        Ok(())
    }

    /// Get config for a workspace; returns a disabled default when missing.
    pub fn get_workspace(&self, path: &Path) -> WorkspaceTeamsConfig {
        let key = normalize_workspace_key(path);
        self.workspaces.get(&key).cloned().unwrap_or_default()
    }

    pub fn set_workspace(&mut self, path: &Path, ws: WorkspaceTeamsConfig) {
        let key = normalize_workspace_key(path);
        self.workspaces.insert(key, ws);
    }
}
