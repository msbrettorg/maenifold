use dashmap::DashMap;
use serde_json::Value;
use std::sync::OnceLock;

static RUNTIME_MODELS: OnceLock<DashMap<String, Vec<String>>> = OnceLock::new();
static RUNTIME_MODES: OnceLock<DashMap<String, Vec<String>>> = OnceLock::new();
static RUNTIME_CONFIG_OPTIONS: OnceLock<DashMap<String, Vec<Value>>> = OnceLock::new();
static RUNTIME_AVAILABLE_COMMANDS: OnceLock<DashMap<String, Vec<Value>>> = OnceLock::new();

fn runtime_models() -> &'static DashMap<String, Vec<String>> {
    RUNTIME_MODELS.get_or_init(DashMap::new)
}

fn runtime_modes() -> &'static DashMap<String, Vec<String>> {
    RUNTIME_MODES.get_or_init(DashMap::new)
}

fn runtime_config_options() -> &'static DashMap<String, Vec<Value>> {
    RUNTIME_CONFIG_OPTIONS.get_or_init(DashMap::new)
}

fn runtime_available_commands() -> &'static DashMap<String, Vec<Value>> {
    RUNTIME_AVAILABLE_COMMANDS.get_or_init(DashMap::new)
}

pub(super) fn clear_all() {
    runtime_models().clear();
    runtime_modes().clear();
    runtime_config_options().clear();
    runtime_available_commands().clear();
}

pub(super) fn clear_session(app_session_id: &str, runtime_key: &str, role_name: &str) {
    runtime_available_commands().remove(&session_role_key(app_session_id, runtime_key, role_name));
}

pub(super) fn remember_runtime_models(runtime_key: &str, mut models: Vec<String>) {
    if models.is_empty() {
        return;
    }
    models.sort_unstable();
    models.dedup();
    runtime_models().insert(runtime_key.to_string(), models);
}

pub(super) fn remember_runtime_modes(runtime_key: &str, mut modes: Vec<String>) {
    if modes.is_empty() {
        return;
    }
    modes.sort_unstable();
    modes.dedup();
    runtime_modes().insert(runtime_key.to_string(), modes);
}

pub fn list_discovered_models(runtime_key: &str) -> Vec<String> {
    runtime_models()
        .get(runtime_key)
        .map(|v| v.clone())
        .unwrap_or_default()
}

pub fn list_discovered_modes(runtime_key: &str) -> Vec<String> {
    runtime_modes()
        .get(runtime_key)
        .map(|v| v.clone())
        .unwrap_or_default()
}

// Config option *definitions* (the schema of what options exist) are scoped per-runtime,
// not per-session-role. Two concurrent sessions on the same runtime share the same
// discovered option definitions (last-write-wins). Config option *values* are stored
// separately per-session-role in app_session_roles.config_options_json.
pub(super) fn remember_runtime_config_options(runtime_key: &str, options: Vec<Value>) {
    if options.is_empty() {
        return;
    }
    runtime_config_options().insert(runtime_key.to_string(), options);
}

pub fn list_discovered_config_options(runtime_key: &str) -> Vec<Value> {
    runtime_config_options()
        .get(runtime_key)
        .map(|v| v.clone())
        .unwrap_or_default()
}

pub(super) fn remember_runtime_available_commands(
    app_session_id: &str,
    runtime_key: &str,
    role_name: &str,
    commands: Vec<Value>,
) {
    runtime_available_commands().insert(
        session_role_key(app_session_id, runtime_key, role_name),
        commands,
    );
}

pub fn list_available_commands(
    app_session_id: &str,
    runtime_key: &str,
    role_name: &str,
) -> Vec<Value> {
    runtime_available_commands()
        .get(&session_role_key(app_session_id, runtime_key, role_name))
        .map(|v| v.clone())
        .unwrap_or_default()
}

fn session_role_key(app_session_id: &str, runtime_key: &str, role_name: &str) -> String {
    format!("{app_session_id}:{runtime_key}:{role_name}")
}
