use super::{ServerMode, resolve_open_url};
use crate::tray_menu::{is_launchd_managed, launchd_service_target};
use crate::tray_viewer::{
    GATE_VIEWER_MENU_TEXT, PIPELINE_VIEWER_MENU_TEXT, is_gate_viewer_event, is_pipeline_viewer_event,
};
use tray_icon::menu::MenuId;

#[test]
fn help_menu_routes_to_issues_url() {
    assert_eq!(
        resolve_open_url(
            &MenuId::new("help"),
            Some(&MenuId::new("help")),
            ServerMode::Http(1234)
        ),
        Some("https://github.com/ma-collective/maenifold-rs/issues".to_string())
    );
}

#[test]
fn unrelated_menu_ids_do_not_open_urls() {
    assert_eq!(
        resolve_open_url(
            &MenuId::new("quit"),
            Some(&MenuId::new("help")),
            ServerMode::Http(1234)
        ),
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

#[test]
fn tray_menu_wires_pipeline_viewer_item() {
    let pipeline_id = MenuId::new("pipeline-viewer");

    assert_eq!(PIPELINE_VIEWER_MENU_TEXT, "Pipeline Viewer");
    assert!(is_pipeline_viewer_event(&pipeline_id, Some(&pipeline_id)));
    assert!(!is_pipeline_viewer_event(
        &MenuId::new("help"),
        Some(&pipeline_id)
    ));
}

#[test]
fn tray_menu_wires_gate_viewer_item() {
    let gate_id = MenuId::new("gate-viewer");

    assert_eq!(GATE_VIEWER_MENU_TEXT, "Gate Board");
    assert!(is_gate_viewer_event(&gate_id, Some(&gate_id)));
    assert!(!is_gate_viewer_event(&MenuId::new("help"), Some(&gate_id)));
}

/// `launchd_service_target()` must match the exact name used in `rebuild.sh`
/// (`launchctl kickstart -k gui/$UID/com.ma-collective.maenifold`). Drift here
/// silently breaks tray-driven restart because launchd will reject an unknown
/// service target.
#[cfg(target_os = "macos")]
#[test]
fn launchd_service_target_uses_canonical_label() {
    let target = launchd_service_target();
    assert!(
        target.starts_with("gui/"),
        "expected gui/<uid>/<label>, got {target}"
    );
    assert!(
        target.ends_with("/com.ma-collective.maenifold"),
        "expected canonical service label, got {target}"
    );
}

/// Tests run from `cargo test`, never under launchd, so this must always
/// return false. If it ever returns true in a test process, the launchctl
/// kickstart path would fire during a test run.
#[test]
fn is_launchd_managed_returns_false_under_cargo_test() {
    assert!(!is_launchd_managed());
}
