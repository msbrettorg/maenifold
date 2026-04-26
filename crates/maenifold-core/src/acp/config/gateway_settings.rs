use serde::{Deserialize, Serialize};

use super::types::McpServerEntry;

fn default_model() -> String {
    "gpt-5.4-mini".to_string()
}

fn default_threshold() -> f64 {
    0.9
}

fn default_catalog_chars() -> u32 {
    24000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySettings {
    #[serde(default)]
    pub intent_resolver: IntentResolverSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentResolverSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_candidates: Option<u32>,
    #[serde(default)]
    pub auto_execute_read_safe: bool,
    #[serde(default = "default_threshold")]
    pub auto_execute_confidence_threshold: f64,
    #[serde(default = "default_catalog_chars")]
    pub max_catalog_characters: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
}

impl IntentResolverSettings {
    pub fn effective_thought_level(&self) -> Option<&str> {
        self.thought_level
            .as_deref()
            .filter(|level| !level.eq_ignore_ascii_case("none"))
    }
}

impl Default for IntentResolverSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            agent: None,
            model: default_model(),
            thought_level: None,
            timeout_seconds: None,
            max_candidates: None,
            auto_execute_read_safe: false,
            auto_execute_confidence_threshold: default_threshold(),
            max_catalog_characters: default_catalog_chars(),
            mcp_servers: Vec::new(),
        }
    }
}
