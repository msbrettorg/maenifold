use super::{ServerMode, resolve_open_url};
use tray_icon::menu::MenuId;

#[test]
fn help_menu_routes_to_issues_url() {
    assert_eq!(
        resolve_open_url(&MenuId::new("help"), Some(&MenuId::new("help")), ServerMode::Http(1234)),
        Some("https://github.com/ma-collective/maenifold-rs/issues".to_string())
    );
}

#[test]
fn unrelated_menu_ids_do_not_open_urls() {
    assert_eq!(
        resolve_open_url(&MenuId::new("quit"), Some(&MenuId::new("help")), ServerMode::Http(1234)),
        None
    );
}

#[test]
fn missing_menu_ids_do_not_open_urls() {
    assert_eq!(
        resolve_open_url(&MenuId::new("help"), None, ServerMode::Http(1234)),
        None
    );
}

#[test]
fn server_mode_port_returns_http_port() {
    assert_eq!(ServerMode::Http(1234).port(), 1234);
}
