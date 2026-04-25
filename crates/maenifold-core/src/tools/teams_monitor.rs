use super::context::{ToolContext, render, required_string};
use crate::acp_config::{AgentsConfig, MaenifoldSettings, TeamsMonitorConfig};
use crate::error::{MaenifoldError, Result};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub fn teams_monitor(context: &ToolContext, payload: &Value) -> Result<String> {
    let action = required_string(payload, "action")?;
    match action {
        "enable" => set_enabled(context, true),
        "disable" => set_enabled(context, false),
        "status" => monitor_status(context),
        "configure" => configure_monitor(context, payload),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown action '{}'. Available: enable, disable, status, configure",
            action
        ))),
    }
}

fn set_enabled(context: &ToolContext, enabled: bool) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root);
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    settings.teams_monitor.enabled = enabled;
    let monitor = settings.teams_monitor.clone();
    config.settings = Some(settings);
    write_agents_config(root, &config)?;
    Ok(render(
        context.output_format,
        status_value(
            if enabled { "enable" } else { "disable" },
            &monitor,
            monitor_connection_state(&monitor),
            if enabled {
                "Teams monitor enabled in maenifold.json."
            } else {
                "Teams monitor disabled in maenifold.json."
            },
        ),
    ))
}

fn monitor_status(context: &ToolContext) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root);
    Ok(render(
        context.output_format,
        status_value(
            "status",
            &monitor,
            monitor_connection_state(&monitor),
            "Teams monitor configuration loaded from maenifold.json.",
        ),
    ))
}

fn configure_monitor(context: &ToolContext, payload: &Value) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root);
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    let monitor = &mut settings.teams_monitor;

    if let Some(keywords) = optional_string_vec(payload, "keywords")? {
        monitor.handler.keywords = keywords;
    }
    if let Some(sender_allow_list) = optional_nullable_string_vec(payload, "senderAllowList")? {
        monitor.handler.sender_allow_list = sender_allow_list;
    }
    if let Some(sender_deny_list) = optional_nullable_string_vec(payload, "senderDenyList")? {
        monitor.handler.sender_deny_list = sender_deny_list;
    }
    if let Some(cooldown_seconds) = optional_u64(payload, "cooldownSeconds")? {
        monitor.handler.cooldown_seconds = cooldown_seconds;
    }
    if let Some(dispatch_agent) = optional_nullable_string(payload, "dispatchAgent")? {
        validate_dispatch_agent(&config, dispatch_agent.as_deref())?;
        monitor.dispatch_agent = dispatch_agent;
    }

    let monitor = monitor.clone();
    config.settings = Some(settings);
    write_agents_config(root, &config)?;

    Ok(render(
        context.output_format,
        status_value(
            "configure",
            &monitor,
            monitor_connection_state(&monitor),
            "Teams monitor configuration updated in maenifold.json.",
        ),
    ))
}

fn load_monitor_config(root: &Path) -> TeamsMonitorConfig {
    AgentsConfig::load(root)
        .settings
        .unwrap_or_else(MaenifoldSettings::default)
        .teams_monitor
}

fn status_value(
    action: &str,
    monitor: &TeamsMonitorConfig,
    connection_state: &'static str,
    message: &str,
) -> Value {
    json!({
        "action": action,
        "enabled": monitor.enabled,
        "connectionState": connection_state,
        "config": {
            "keywords": monitor.handler.keywords,
            "senderAllowList": monitor.handler.sender_allow_list,
            "senderDenyList": monitor.handler.sender_deny_list,
            "cooldownSeconds": monitor.handler.cooldown_seconds,
            "dispatchAgent": monitor.dispatch_agent,
        },
        "message": message,
    })
}

fn monitor_connection_state(config: &TeamsMonitorConfig) -> &'static str {
    if !config.enabled {
        "disabled"
    } else {
        "not_connected"
    }
}

fn optional_string_vec(payload: &Value, key: &'static str) -> Result<Option<Vec<String>>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an array")))?;
    array
        .iter()
        .map(|entry| {
            entry.as_str().map(|text| text.to_string()).ok_or_else(|| {
                MaenifoldError::InvalidArgument(format!("'{key}' entries must be strings"))
            })
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn optional_nullable_string_vec(
    payload: &Value,
    key: &'static str,
) -> Result<Option<Option<Vec<String>>>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    optional_string_vec(payload, key).map(|value| value.map(Some))
}

fn optional_nullable_string(
    payload: &Value,
    key: &'static str,
) -> Result<Option<Option<String>>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    value
        .as_str()
        .map(|text| Some(Some(text.to_string())))
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a string")))
}

fn optional_u64(payload: &Value, key: &'static str) -> Result<Option<u64>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an integer")))
}

fn validate_dispatch_agent(config: &AgentsConfig, dispatch_agent: Option<&str>) -> Result<()> {
    let Some(dispatch_agent) = dispatch_agent else {
        return Ok(());
    };
    if config.agents.contains_key(dispatch_agent) {
        Ok(())
    } else {
        Err(MaenifoldError::InvalidArgument(format!(
            "dispatch_agent '{}' is not configured in maenifold.json",
            dispatch_agent
        )))
    }
}

fn write_agents_config(root: &Path, config: &AgentsConfig) -> Result<()> {
    let path = root.join("maenifold.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_vec_pretty(config)?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, &path).map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to replace '{}': {}",
            path.display(),
            error
        ))
    })
}
