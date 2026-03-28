mod agent;
mod commands;
mod config;
mod support;

use agent::AgentManager;
use config::ConfigManager;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use support::{
    build_icon, hide_main_window, is_login_enabled, remove_launch_agent, show_main_window,
    update_login_menu_item, write_launch_agent,
};
use tauri::{
    Manager, State, WindowEvent,
    menu::{CheckMenuItem, MenuBuilder, MenuEvent, MenuId},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

const MAIN_WINDOW_LABEL: &str = "main";
const OPEN_WINDOW_MENU_ID: &str = "tray-open-window";
const OPEN_AT_LOGIN_MENU_ID: &str = "tray-open-at-login";
const HELP_MENU_ID: &str = "tray-help";
const QUIT_MENU_ID: &str = "tray-quit";
const HELP_URL: &str = "https://github.com/ma-collective/maenifold-rs/issues";

pub(crate) struct AppState {
    config_manager: Arc<RwLock<Option<ConfigManager>>>,
    agent_manager: AgentManager,
    tray_icon: Arc<Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>>,
}

fn spawn_relay_server(
    port: u16,
) -> std::io::Result<(CancellationToken, std::thread::JoinHandle<i32>)> {
    let shutdown_token = CancellationToken::new();
    let server_shutdown_token = shutdown_token.clone();
    let server_handle = std::thread::Builder::new()
        .name("maenifold-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    eprintln!("Error creating async runtime: {err}");
                    return 1i32;
                }
            };

            let result = rt.block_on(async move {
                tokio::select! {
                    result = async {
                        maenifold_core::mcp::MaenifoldMcpServer::serve_web(port).await
                    } => result,
                    _ = server_shutdown_token.cancelled() => Ok(()),
                }
            });

            match result {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("Server error: {err}");
                    1
                }
            }
        })?;

    Ok((shutdown_token, server_handle))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_app(port: u16) -> i32 {
    let (server_shutdown_token, server_handle) = match spawn_relay_server(port) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("Failed to spawn relay server thread: {err}");
            return 1;
        }
    };

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
                Ok(config_manager) => *state.config_manager.write() = Some(config_manager),
                Err(err) => eprintln!("Failed to initialize config manager: {err}"),
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
                            write_launch_agent(port)
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
            commands::get_config,
            commands::reload_config,
            commands::get_config_path,
            commands::spawn_agent,
            commands::send_to_agent,
            commands::kill_agent,
            commands::list_running_agents,
            commands::add_agent,
            commands::remove_agent,
            commands::update_agent,
            commands::get_machine_id
        ])
        .build(tauri::generate_context!());

    let exit_code = match app {
        Ok(app) => {
            app.run(|_, _| {});
            0
        }
        Err(err) => {
            eprintln!("error while running tauri application: {err}");
            1
        }
    };

    server_shutdown_token.cancel();
    let server_exit_code = server_handle.join().unwrap_or(1);

    if exit_code != 0 {
        exit_code
    } else {
        server_exit_code
    }
}
