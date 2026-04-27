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
    let mut value = serde_json::to_value(settings).unwrap_or_else(|_| json!({}));
    if let Some(teams_monitor) = value.get_mut("teamsMonitor").and_then(Value::as_object_mut)
        && let Some(identity) = settings.teams_monitor.identity.as_ref()
    {
        teams_monitor.insert(
            "identity".to_string(),
            serde_json::to_value(identity.to_public()).unwrap_or_else(|_| json!(null)),
        );
    }
    value
}
