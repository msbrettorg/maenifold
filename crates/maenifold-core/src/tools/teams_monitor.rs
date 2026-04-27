use super::context::{ToolContext, render, required_string};
use super::teams_monitor_auth::{detect_identity, refresh, restore_on_enable, sign_out};
use super::teams_monitor_helpers::status_value;
use crate::acp_config::{AgentsConfig, MaenifoldSettings, TeamsMonitorConfig};
use crate::error::{MaenifoldError, Result};
use crate::teams::{
    MessageHandlerConfig, TokenInfo, TrouterClientConfig, set_teams_disabled, set_teams_error,
    supervisor::{TrouterSupervisor, default_dispatch_fn},
    teams_runtime_snapshot, token_provider::TeamsTokenProvider,
};
use serde_json::Value;
use std::fs;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static SUPERVISOR: LazyLock<Mutex<Option<TrouterSupervisor>>> = LazyLock::new(|| Mutex::new(None));
static SUPERVISOR_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build Teams monitor supervisor runtime: {error}"))
});

pub fn teams_monitor(context: &ToolContext, payload: &Value) -> Result<String> {
    let action = required_string(payload, "action")?;
    match action {
        "enable" => set_enabled(context, true),
        "disable" => set_enabled(context, false),
        "status" => monitor_status(context),
        "configure" => configure_monitor(context, payload),
        "detect_identity" => detect_identity(context),
        "refresh" => refresh(context),
        "sign_out" => sign_out(context),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown action '{}'. Available: enable, disable, status, configure, detect_identity, refresh, sign_out",
            action
        ))),
    }
}

fn set_enabled(context: &ToolContext, enabled: bool) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    if enabled {
        if let Err(error) = restore_on_enable(&mut settings.teams_monitor, root) {
            set_teams_error(error.to_string());
            return Err(error);
        }
        settings.teams_monitor.enabled = true;
        let monitor = settings.teams_monitor.clone();
        config.settings = Some(settings);
        write_agents_config(root, &config)?;
        let runtime = teams_runtime_snapshot();
        return Ok(render(
            context.output_format,
            status_value(
                "enable",
                &monitor,
                &runtime,
                "Teams monitor enabled using the signed-in Teams environment.",
            ),
        ));
    } else {
        settings.teams_monitor.enabled = false;
        let monitor = settings.teams_monitor.clone();
        config.settings = Some(settings);
        write_agents_config(root, &config)?;
        stop_monitor_supervisor()?;
        let runtime = teams_runtime_snapshot();
        return Ok(render(
            context.output_format,
            status_value(
                "disable",
                &monitor,
                &runtime,
                "Teams monitor disabled in maenifold.json.",
            ),
        ));
    }
}

pub(super) fn start_monitor_supervisor(
    monitor: &TeamsMonitorConfig,
    initial_token: TokenInfo,
    maenifold_root: &Path,
) -> Result<()> {
    if let Some(mut supervisor) = take_supervisor()? {
        run_on_supervisor_runtime(supervisor.stop())?;
    }

    let handler_config = handler_config_from_monitor(monitor);
    let trouter_config = trouter_config_from_monitor(monitor);
    let dispatch_agent = monitor.dispatch_agent.clone();
    let provider = Arc::new(TeamsTokenProvider::new());
    let fallback_refresh =
        Duration::from_secs(monitor.connection.token_refresh_minutes.max(1) * 60);
    // T-238.56: <root>/runtime/teams-monitor.lock — fd-lock RAII singleton.
    let lock_path = maenifold_root.join("runtime").join("teams-monitor.lock");
    let supervisor = run_on_supervisor_runtime(async move {
        TrouterSupervisor::start(
            initial_token,
            provider,
            fallback_refresh,
            trouter_config,
            handler_config,
            dispatch_agent,
            default_dispatch_fn(),
            lock_path,
        )
    })??;
    *SUPERVISOR
        .lock()
        .map_err(|_| MaenifoldError::Message("Teams supervisor lock poisoned".to_string()))? =
        Some(supervisor);
    Ok(())
}

fn stop_monitor_supervisor() -> Result<()> {
    if let Some(mut supervisor) = take_supervisor()? {
        run_on_supervisor_runtime(supervisor.stop())?;
    }
    set_teams_disabled();
    Ok(())
}

fn take_supervisor() -> Result<Option<TrouterSupervisor>> {
    Ok(SUPERVISOR
        .lock()
        .map_err(|_| MaenifoldError::Message("Teams supervisor lock poisoned".to_string()))?
        .take())
}

fn supervisor_runtime() -> Result<&'static Runtime> {
    SUPERVISOR_RUNTIME
        .as_ref()
        .map_err(|message| MaenifoldError::Message(message.clone()))
}

fn run_on_supervisor_runtime<F, T>(future: F) -> Result<T>
where
    F: Future<Output = T>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "Teams supervisor requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                ));
            }
            task::block_in_place(|| supervisor_runtime().map(|runtime| runtime.block_on(future)))
        }
        Err(_) => Ok(supervisor_runtime()?.block_on(future)),
    }
}

fn monitor_status(context: &ToolContext) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root)?;
    let runtime = teams_runtime_snapshot();
    Ok(render(
        context.output_format,
        status_value(
            "status",
            &monitor,
            &runtime,
            "Teams monitor configuration loaded from maenifold.json.",
        ),
    ))
}

fn configure_monitor(context: &ToolContext, payload: &Value) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    let monitor = &mut settings.teams_monitor;

    if let Some(keyword) = optional_string(payload, "keyword")? {
        monitor.keyword = keyword;
    }
    if let Some(listen_scope) = optional_string(payload, "listenScope")? {
        monitor.listen_scope = listen_scope;
    }
    if let Some(monitored_conversations) = optional_string_vec(payload, "monitoredConversations")? {
        monitor.monitored_conversations = monitored_conversations;
    }
    if let Some(reply_behavior) = optional_string(payload, "replyBehavior")? {
        monitor.reply_behavior = reply_behavior;
    }
    if let Some(reply_prefix) = optional_string(payload, "replyPrefix")? {
        monitor.reply_prefix = reply_prefix;
    }
    if let Some(dispatch_timeout_minutes) = optional_u64(payload, "dispatchTimeoutMinutes")? {
        monitor.dispatch_timeout_minutes = dispatch_timeout_minutes;
    }
    if let Some(chatsvc_region) = optional_string(payload, "chatsvcRegion")? {
        monitor.connection.chatsvc_region = chatsvc_region;
    }
    if let Some(trouter_gateway) = optional_string(payload, "trouterGateway")? {
        monitor.connection.trouter_gateway = trouter_gateway;
    }
    if let Some(registrar_url) = optional_string(payload, "registrarUrl")? {
        monitor.connection.registrar_url = registrar_url;
    }
    if let Some(app_id) = optional_string(payload, "appId")? {
        monitor.connection.app_id = app_id;
    }
    if let Some(reconnect_delay_seconds) = optional_u64(payload, "reconnectDelaySeconds")? {
        monitor.connection.reconnect_delay_seconds = reconnect_delay_seconds;
    }
    if let Some(max_reconnect_delay_seconds) = optional_u64(payload, "maxReconnectDelaySeconds")? {
        monitor.connection.max_reconnect_delay_seconds = max_reconnect_delay_seconds;
    }
    if let Some(token_refresh_minutes) = optional_u64(payload, "tokenRefreshMinutes")? {
        monitor.connection.token_refresh_minutes = token_refresh_minutes;
    }
    if let Some(heartbeat_interval_seconds) = optional_u64(payload, "heartbeatIntervalSeconds")? {
        monitor.connection.heartbeat_interval_seconds = heartbeat_interval_seconds;
    }
    if let Some(dispatch_agent) = optional_nullable_string(payload, "dispatchAgent")? {
        validate_dispatch_agent(&config, dispatch_agent.as_deref())?;
        monitor.dispatch_agent = dispatch_agent;
    }
    if monitor.enabled {
        restore_on_enable(monitor, root)?;
    }

    let monitor = monitor.clone();
    config.settings = Some(settings);
    write_agents_config(root, &config)?;

    Ok(render(
        context.output_format,
        status_value(
            "configure",
            &monitor,
            &teams_runtime_snapshot(),
            "Teams monitor configuration updated in maenifold.json.",
        ),
    ))
}

fn handler_config_from_monitor(monitor: &TeamsMonitorConfig) -> MessageHandlerConfig {
    MessageHandlerConfig {
        keywords: vec![monitor.keyword.clone()],
        listen_scope: monitor.listen_scope.clone(),
        monitored_conversations: monitor.monitored_conversations.clone(),
        self_mri: monitor
            .identity
            .as_ref()
            .map(|identity| identity.mri.clone()),
        reply_behavior: monitor.reply_behavior.clone(),
        reply_prefix: monitor.reply_prefix.clone(),
        chatsvc_region: monitor.connection.chatsvc_region.clone(),
        dispatch_timeout_minutes: monitor.dispatch_timeout_minutes,
        sender_allow_list: None,
        sender_deny_list: None,
        cooldown_seconds: 0,
    }
}

fn trouter_config_from_monitor(monitor: &TeamsMonitorConfig) -> TrouterClientConfig {
    TrouterClientConfig {
        gateway: monitor.connection.trouter_gateway.clone(),
        registrar_url: monitor.connection.registrar_url.clone(),
        app_id: monitor.connection.app_id.clone(),
        heartbeat: std::time::Duration::from_secs(monitor.connection.heartbeat_interval_seconds),
        initial_backoff: std::time::Duration::from_secs(monitor.connection.reconnect_delay_seconds),
        max_backoff: std::time::Duration::from_secs(monitor.connection.max_reconnect_delay_seconds),
    }
}

pub(super) fn load_monitor_config(root: &Path) -> Result<TeamsMonitorConfig> {
    Ok(AgentsConfig::load(root)?
        .settings
        .unwrap_or_else(MaenifoldSettings::default)
        .teams_monitor)
}

fn optional_string(payload: &Value, key: &'static str) -> Result<Option<String>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|text| Some(text.to_string()))
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a string")))
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

fn optional_nullable_string(payload: &Value, key: &'static str) -> Result<Option<Option<String>>> {
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

pub(super) fn write_agents_config(root: &Path, config: &AgentsConfig) -> Result<()> {
    let path = root.join("maenifold.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_vec_pretty(config)?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, &path).map_err(|error| {
        MaenifoldError::Message(format!("Failed to replace '{}': {}", path.display(), error))
    })
}

#[cfg(test)]
#[path = "teams_monitor_tests.rs"]
mod tests;
