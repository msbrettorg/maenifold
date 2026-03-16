mod agent;
mod config;

use agent::{AgentInstance, AgentManager};
use config::{AgentConfig, AgentsConfig, ConfigManager};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use tauri::{
    AppHandle, Manager, State, Window, WindowEvent,
    image::Image,
    menu::{CheckMenuItem, MenuBuilder, MenuEvent, MenuId},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW_LABEL: &str = "main";
const OPEN_WINDOW_MENU_ID: &str = "tray-open-window";
const OPEN_AT_LOGIN_MENU_ID: &str = "tray-open-at-login";
const HELP_MENU_ID: &str = "tray-help";
const QUIT_MENU_ID: &str = "tray-quit";
const HELP_URL: &str = "https://github.com/ma-collective/maenifold-rs/issues";

struct AppState {
    config_manager: Arc<RwLock<Option<ConfigManager>>>,
    agent_manager: AgentManager,
    tray_icon: Arc<Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>>,
}

fn build_icon() -> Image<'static> {
    let rgba = include_bytes!("../../maenifold-core/src/tray_icon.rgba");
    Image::new_owned(rgba.to_vec(), 64, 64)
}

fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library/LaunchAgents/com.ma-collective.maenifold.plist")
}

fn is_login_enabled() -> bool {
    launch_agent_path().exists()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_launch_agent(port: u16) -> std::io::Result<()> {
    let path = launch_agent_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let exe_path = std::env::current_exe()?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ma-collective.maenifold</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>--app</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        exe_path = xml_escape(&exe_path.display().to_string()),
        port = port,
    );

    std::fs::write(path, plist)
}

fn remove_launch_agent() -> std::io::Result<()> {
    let path = launch_agent_path();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn update_login_menu_item(item: &CheckMenuItem<tauri::Wry>) {
    if let Err(err) = item.set_checked(is_login_enabled()) {
        eprintln!("Failed to update Open at Login menu state: {err}");
    }
}

fn show_main_window<R: tauri::Runtime, M: Manager<R>>(manager: &M) {
    #[cfg(target_os = "macos")]
    if let Err(err) = manager
        .app_handle()
        .set_activation_policy(tauri::ActivationPolicy::Regular)
    {
        eprintln!("Failed to show dock icon: {err}");
    }

    match manager.get_webview_window(MAIN_WINDOW_LABEL) {
        Some(window) => {
            if let Err(err) = window.unminimize() {
                eprintln!("Failed to unminimize main window: {err}");
            }
            if let Err(err) = window.show() {
                eprintln!("Failed to show main window: {err}");
            }
            if let Err(err) = window.set_focus() {
                eprintln!("Failed to focus main window: {err}");
            }
        }
        None => eprintln!("Main window '{MAIN_WINDOW_LABEL}' not found"),
    }
}

fn hide_main_window<R: tauri::Runtime>(window: &Window<R>) {
    if let Err(err) = window.hide() {
        eprintln!("Failed to hide main window: {err}");
    }

    #[cfg(target_os = "macos")]
    if let Err(err) = window
        .app_handle()
        .set_activation_policy(tauri::ActivationPolicy::Accessory)
    {
        eprintln!("Failed to hide dock icon: {err}");
    }
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AgentsConfig, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .map(|cm| cm.get_config())
        .ok_or_else(|| "Config manager not initialized".to_string())
}

#[tauri::command]
fn reload_config(state: State<AppState>) -> Result<AgentsConfig, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .map(|cm| cm.reload())
        .ok_or_else(|| "Config manager not initialized".to_string())?
}

#[tauri::command]
fn get_config_path(state: State<AppState>) -> Result<String, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .map(|cm| cm.get_config_path().to_string_lossy().to_string())
        .ok_or_else(|| "Config manager not initialized".to_string())
}

#[tauri::command]
fn spawn_agent(
    name: String,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<AgentInstance, String> {
    let config_manager = state.config_manager.read();
    let config = config_manager
        .as_ref()
        .ok_or_else(|| "Config manager not initialized".to_string())?
        .get_config();

    let agent_config = config
        .agents
        .get(&name)
        .ok_or_else(|| format!("Agent '{name}' not found in config"))?;

    state
        .agent_manager
        .spawn_agent(name, agent_config, app_handle)
}

#[tauri::command]
fn send_to_agent(agent_id: String, message: String, state: State<AppState>) -> Result<(), String> {
    state.agent_manager.send_message(&agent_id, &message)
}

#[tauri::command]
fn kill_agent(agent_id: String, state: State<AppState>) -> Result<(), String> {
    state.agent_manager.kill_agent(&agent_id)
}

#[tauri::command]
fn list_running_agents(state: State<AppState>) -> Vec<String> {
    state.agent_manager.list_running_agents()
}

#[tauri::command]
fn add_agent(
    name: String,
    command: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    state: State<AppState>,
) -> Result<AgentsConfig, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .ok_or_else(|| "Config manager not initialized".to_string())?
        .add_agent(name, AgentConfig { command, args, env })
}

#[tauri::command]
fn remove_agent(name: String, state: State<AppState>) -> Result<AgentsConfig, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .ok_or_else(|| "Config manager not initialized".to_string())?
        .remove_agent(&name)
}

#[tauri::command]
fn update_agent(
    name: String,
    command: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    state: State<AppState>,
) -> Result<AgentsConfig, String> {
    let config_manager = state.config_manager.read();
    config_manager
        .as_ref()
        .ok_or_else(|| "Config manager not initialized".to_string())?
        .update_agent(name, AgentConfig { command, args, env })
}

#[tauri::command]
fn get_machine_id() -> Result<String, String> {
    machine_uid::get().map_err(|e| format!("Failed to get machine ID: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_app(_port: u16) -> i32 {
    let app_state = AppState {
        config_manager: Arc::new(RwLock::new(None)),
        agent_manager: AgentManager::new(),
        tray_icon: Arc::new(Mutex::new(None)),
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .on_window_event(|window, event| {
            if window.label() == MAIN_WINDOW_LABEL
                && matches!(event, WindowEvent::CloseRequested { .. })
            {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                hide_main_window(window);
            }
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let state: State<AppState> = app.state();

            match ConfigManager::new(&app_handle) {
                Ok(config_manager) => {
                    *state.config_manager.write() = Some(config_manager);
                }
                Err(err) => {
                    eprintln!("Failed to initialize config manager: {err}");
                }
            }

            let open_at_login = CheckMenuItem::with_id(
                app,
                OPEN_AT_LOGIN_MENU_ID,
                "Open at Login",
                true,
                is_login_enabled(),
                None::<&str>,
            )?;

            let menu = MenuBuilder::new(app)
                .item(&tauri::menu::MenuItem::with_id(
                    app,
                    OPEN_WINDOW_MENU_ID,
                    "Open Window",
                    true,
                    None::<&str>,
                )?)
                .item(&open_at_login)
                .item(&tauri::menu::MenuItem::with_id(
                    app,
                    HELP_MENU_ID,
                    "Help",
                    true,
                    None::<&str>,
                )?)
                .separator()
                .item(&tauri::menu::MenuItem::with_id(
                    app,
                    QUIT_MENU_ID,
                    "Quit",
                    true,
                    None::<&str>,
                )?)
                .build()?;

            let login_toggle_item = open_at_login.clone();
            let tray_icon = TrayIconBuilder::with_id("maenifold-tray")
                .icon(build_icon())
                .icon_as_template(true)
                .tooltip("maenifold")
                .show_menu_on_left_click(false)
                .menu(&menu)
                .on_menu_event(move |app, event: MenuEvent| match event.id() {
                    id if *id == MenuId::from(OPEN_WINDOW_MENU_ID) => show_main_window(app),
                    id if *id == MenuId::from(OPEN_AT_LOGIN_MENU_ID) => {
                        let result = if is_login_enabled() {
                            remove_launch_agent()
                        } else {
                            write_launch_agent(_port)
                        };

                        if let Err(err) = result {
                            eprintln!("Failed to toggle Open at Login: {err}");
                        }

                        update_login_menu_item(&login_toggle_item);
                    }
                    id if *id == MenuId::from(HELP_MENU_ID) => {
                        if let Err(err) = app.opener().open_url(HELP_URL, None::<&str>) {
                            eprintln!("Failed to open help URL: {err}");
                        }
                    }
                    id if *id == MenuId::from(QUIT_MENU_ID) => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            *state.tray_icon.lock() = Some(tray_icon);
            show_main_window(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            reload_config,
            get_config_path,
            spawn_agent,
            send_to_agent,
            kill_agent,
            list_running_agents,
            add_agent,
            remove_agent,
            update_agent,
            get_machine_id
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => {
            app.run(|_, _| {});
            0
        }
        Err(err) => {
            eprintln!("error while running tauri application: {err}");
            1
        }
    }
}
