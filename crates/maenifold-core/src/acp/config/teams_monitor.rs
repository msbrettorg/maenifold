use serde::{Deserialize, Serialize};

fn default_keyword() -> String {
    "@maenifold".to_string()
}

fn default_listen_scope() -> String {
    "wildcard".to_string()
}

fn default_monitored_conversations() -> Vec<String> {
    vec!["48:notes".to_string()]
}

fn default_reply_behavior() -> String {
    "sourceConversation".to_string()
}

fn default_reply_prefix() -> String {
    "maenifold: ".to_string()
}

fn default_dispatch_timeout_minutes() -> u64 {
    10
}

fn default_trouter_gateway() -> String {
    "go-msit.trouter.teams.microsoft.com".to_string()
}

fn default_registrar_url() -> String {
    "https://teams.cloud.microsoft/registrar/prod/V2/registrations".to_string()
}

fn default_app_id() -> String {
    "TeamsCDLWebWorker".to_string()
}

fn default_reconnect_delay_seconds() -> u64 {
    5
}

fn default_max_reconnect_delay_seconds() -> u64 {
    300
}

fn default_token_refresh_minutes() -> u64 {
    50
}

fn default_heartbeat_interval_seconds() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsMonitorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub identity: Option<TeamsMonitorIdentityConfig>,
    #[serde(default = "default_keyword")]
    pub keyword: String,
    #[serde(default = "default_listen_scope")]
    pub listen_scope: String,
    #[serde(default = "default_monitored_conversations")]
    pub monitored_conversations: Vec<String>,
    #[serde(default = "default_reply_behavior")]
    pub reply_behavior: String,
    #[serde(default = "default_reply_prefix")]
    pub reply_prefix: String,
    #[serde(default = "default_dispatch_timeout_minutes")]
    pub dispatch_timeout_minutes: u64,
    #[serde(default)]
    pub dispatch_agent: Option<String>,
    #[serde(default)]
    pub connection: TeamsMonitorConnectionConfig,
}

impl Default for TeamsMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            identity: None,
            keyword: default_keyword(),
            listen_scope: default_listen_scope(),
            monitored_conversations: default_monitored_conversations(),
            reply_behavior: default_reply_behavior(),
            reply_prefix: default_reply_prefix(),
            dispatch_timeout_minutes: default_dispatch_timeout_minutes(),
            dispatch_agent: None,
            connection: TeamsMonitorConnectionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsMonitorIdentityConfig {
    pub mri: String,
    pub object_id: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub upn: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    pub expires_at: String,
    #[serde(default)]
    pub chatsvc_region: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsMonitorConnectionConfig {
    #[serde(default = "default_trouter_gateway")]
    pub trouter_gateway: String,
    #[serde(default = "default_registrar_url")]
    pub registrar_url: String,
    #[serde(default = "default_app_id")]
    pub app_id: String,
    #[serde(default = "default_reconnect_delay_seconds")]
    pub reconnect_delay_seconds: u64,
    #[serde(default = "default_max_reconnect_delay_seconds")]
    pub max_reconnect_delay_seconds: u64,
    #[serde(default = "default_token_refresh_minutes")]
    pub token_refresh_minutes: u64,
    #[serde(default = "default_heartbeat_interval_seconds")]
    pub heartbeat_interval_seconds: u64,
    #[serde(default)]
    pub chatsvc_region: String,
}

impl Default for TeamsMonitorConnectionConfig {
    fn default() -> Self {
        Self {
            trouter_gateway: default_trouter_gateway(),
            registrar_url: default_registrar_url(),
            app_id: default_app_id(),
            reconnect_delay_seconds: default_reconnect_delay_seconds(),
            max_reconnect_delay_seconds: default_max_reconnect_delay_seconds(),
            token_refresh_minutes: default_token_refresh_minutes(),
            heartbeat_interval_seconds: default_heartbeat_interval_seconds(),
            chatsvc_region: String::new(),
        }
    }
}
