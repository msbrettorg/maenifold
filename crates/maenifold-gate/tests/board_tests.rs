#![cfg(feature = "app")]

use axum::body::to_bytes;
use axum::http::{StatusCode, header};
use maenifold_gate::{board, tray};
use tray_icon::Icon;

const BOARD_HTML: &str = include_str!("../src/board/index.html");
const TRACE_HTML: &str = include_str!("../src/board/trace.html");
const TRAY_SOURCE: &str = include_str!("../src/tray.rs");
const TRAY_ICON_RGBA: &[u8] = include_bytes!("../src/tray_icon.rgba");

#[tokio::test]
async fn board_index_serves_embedded_html_with_no_cache() {
    let response = board::index().await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-cache"
    );

    let body_bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body = String::from_utf8_lossy(&body_bytes);
    assert!(body.contains("<main class=\"board\" id=\"board\"></main>"));
    assert!(body.contains("const COLUMNS = [\"pending\", \"swe\", \"red\", \"blue\", \"done\"];"));
    assert!(body.contains("function renderCards(tasks)"));
}

#[tokio::test]
async fn board_trace_serves_embedded_trace_html_with_no_cache() {
    let response = board::trace().await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-cache"
    );

    let body_bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body = String::from_utf8_lossy(&body_bytes);
    assert!(body.contains("<main class=\"list\" id=\"trace-list\">"));
    assert!(body.contains("Waiting for MCP traffic"));
}

#[test]
fn board_groups_tasks_by_pipeline_stage_columns() {
    assert!(
        BOARD_HTML.contains("const COLUMNS = [\"pending\", \"swe\", \"red\", \"blue\", \"done\"];")
    );
    assert!(BOARD_HTML.contains("return COLUMNS.includes(status) ? status : \"pending\";"));
    assert!(
        BOARD_HTML.contains("const tasks = state.tasks.filter(task => task.column === column);")
    );
    assert!(
        BOARD_HTML
            .contains("const body = column === \"done\" ? renderDone(tasks) : renderCards(tasks);")
    );
}

#[test]
fn board_handles_empty_tasks_and_missing_pipeline_stages() {
    assert!(
        BOARD_HTML.contains("if (!tasks.length) return '<div class=\"empty\">No tasks</div>';")
    );
    assert!(
        BOARD_HTML
            .contains("if (!tasks.length) return '<div class=\"empty\">No completed tasks</div>';")
    );
    assert!(BOARD_HTML.contains("for (const stage of task.stages || [])"));
    assert!(BOARD_HTML.contains("verdicts || '<span class=\"meta-chip\">no verdicts</span>'"));
}

#[test]
fn board_escapes_html_when_rendering_task_fields_and_sections() {
    assert!(BOARD_HTML.contains("function escapeHtml(value)"));
    for escaped_field in [
        "${escapeHtml(task.id)}",
        "${escapeHtml(task.title)}",
        "${escapeHtml(task.status)}",
        "${escapeHtml(task.location)}",
        "${escapeHtml(task.gateName)}",
        "${escapeHtml(section.heading)}",
        "${escapeHtml(section.stage)}",
        "${escapeHtml(section.body)}",
    ] {
        assert!(
            BOARD_HTML.contains(escaped_field),
            "missing escaped field: {escaped_field}"
        );
    }
}

#[test]
fn board_assets_match_embedded_source_files() {
    assert!(BOARD_HTML.contains("<title>Gate Board</title>"));
    assert!(TRACE_HTML.contains("<title>MCP Trace — Gate</title>"));
}

#[test]
fn tray_icon_rgba_is_valid_for_icon_generation() {
    assert_eq!(TRAY_ICON_RGBA.len(), 64 * 64 * 4);
    assert!(Icon::from_rgba(TRAY_ICON_RGBA.to_vec(), 64, 64).is_ok());
}

#[test]
fn tray_menu_specs_cover_expected_labels() {
    let disabled_login = tray::tray_menu_specs(false);
    assert_eq!(disabled_login[0].label, "Kanban Board");
    assert_eq!(disabled_login[1].label, "Trace");
    assert_eq!(disabled_login.last().unwrap().label, "Quit");
    assert_eq!(disabled_login[2].label, "Open at Login");
    assert_eq!(disabled_login[2].enabled, cfg!(target_os = "macos"));

    let enabled_login = tray::tray_menu_specs(true);
    assert_eq!(enabled_login[2].label, "✓ Open at Login");
    assert_eq!(
        enabled_login.iter().any(|item| item.label == "Restart"),
        cfg!(target_os = "macos")
    );
}

#[test]
fn tray_launch_agent_path_generation_is_stable() {
    let home = std::path::Path::new("/tmp/example-home");
    let path = tray::launch_agent_path_from(Some(home));
    assert_eq!(
        path,
        home.join("Library/LaunchAgents/com.ma-collective.gate.plist")
    );
    assert_eq!(
        tray::launch_agent_path_from(None),
        std::path::Path::new(".").join("Library/LaunchAgents/com.ma-collective.gate.plist")
    );
}

#[test]
fn tray_source_keeps_pure_utility_hooks_for_login_and_restart() {
    assert!(TRAY_SOURCE.contains("fn build_icon() -> Icon"));
    assert!(TRAY_SOURCE.contains("pub fn launch_agent_path_from"));
    assert!(TRAY_SOURCE.contains("pub fn login_menu_text(enabled: bool) -> &'static str"));
    assert!(TRAY_SOURCE.contains("fn restart_daemon(service_label: &str)"));
}
