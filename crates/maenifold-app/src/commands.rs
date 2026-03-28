use crate::{
    AppState,
    agent::AgentInstance,
    config::{AgentConfig, AgentsConfig, ConfigManager},
};
use tauri::{AppHandle, State};

fn with_config_manager<T>(
    state: &State<AppState>,
    f: impl FnOnce(&ConfigManager) -> Result<T, String>,
) -> Result<T, String> {
    let config_manager = state.config_manager.read();
    let config_manager = config_manager
        .as_ref()
        .ok_or_else(|| "Config manager not initialized".to_string())?;
    f(config_manager)
}

#[tauri::command]
pub(crate) fn get_config(state: State<AppState>) -> Result<AgentsConfig, String> {
    with_config_manager(&state, |config_manager| Ok(config_manager.get_config()))
}

#[tauri::command]
pub(crate) fn reload_config(state: State<AppState>) -> Result<AgentsConfig, String> {
    with_config_manager(&state, ConfigManager::reload)
}

#[tauri::command]
pub(crate) fn get_config_path(state: State<AppState>) -> Result<String, String> {
    with_config_manager(&state, |config_manager| {
        Ok(config_manager
            .get_config_path()
            .to_string_lossy()
            .to_string())
    })
}

#[tauri::command]
pub(crate) fn spawn_agent(
    name: String,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<AgentInstance, String> {
    let config = with_config_manager(&state, |config_manager| Ok(config_manager.get_config()))?;
    let agent_config = config
        .agents
        .get(&name)
        .ok_or_else(|| format!("Agent '{name}' not found in config"))?;

    state
        .agent_manager
        .spawn_agent(name, agent_config, app_handle)
}

#[tauri::command]
pub(crate) fn send_to_agent(
    agent_id: String,
    message: String,
    state: State<AppState>,
) -> Result<(), String> {
    state.agent_manager.send_message(&agent_id, &message)
}

#[tauri::command]
pub(crate) fn kill_agent(agent_id: String, state: State<AppState>) -> Result<(), String> {
    state.agent_manager.kill_agent(&agent_id)
}

#[tauri::command]
pub(crate) fn list_running_agents(state: State<AppState>) -> Vec<String> {
    state.agent_manager.list_running_agents()
}

#[tauri::command]
pub(crate) fn add_agent(
    name: String,
    command: String,
    state: State<AppState>,
) -> Result<AgentsConfig, String> {
    with_config_manager(&state, |config_manager| {
        config_manager.add_agent(name, AgentConfig { command })
    })
}

#[tauri::command]
pub(crate) fn remove_agent(name: String, state: State<AppState>) -> Result<AgentsConfig, String> {
    with_config_manager(&state, |config_manager| config_manager.remove_agent(&name))
}

#[tauri::command]
pub(crate) fn update_agent(
    name: String,
    command: String,
    state: State<AppState>,
) -> Result<AgentsConfig, String> {
    with_config_manager(&state, |config_manager| {
        config_manager.update_agent(name, AgentConfig { command })
    })
}

#[tauri::command]
pub(crate) fn get_machine_id() -> Result<String, String> {
    machine_uid::get().map_err(|error| format!("Failed to get machine ID: {error}"))
}
