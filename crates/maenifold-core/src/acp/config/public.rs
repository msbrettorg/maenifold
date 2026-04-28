use serde_json::{Value, json};

use super::loader::AgentsConfig;
use super::settings::MaenifoldSettings;

pub fn public_agents_config_value(config: &AgentsConfig) -> Value {
    let mut value = serde_json::to_value(config).unwrap_or_else(|_| json!({}));
    if let Some(settings) = config.settings.as_ref() {
        if let Some(map) = value.as_object_mut() {
            map.insert("settings".to_string(), public_settings_value(settings));
        }
    }
    value
}

pub fn public_settings_value(settings: &MaenifoldSettings) -> Value {
    let _ = settings;
    serde_json::to_value(settings).unwrap_or_else(|_| json!({}))
}
