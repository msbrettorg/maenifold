use super::{is_pipeline_viewer_event, PIPELINE_VIEWER_MENU_TEXT};
use tray_icon::menu::MenuId;

#[test]
fn pipeline_viewer_menu_event_matches_only_configured_id() {
    let viewer_id = MenuId::new("pipeline-viewer");
    assert!(is_pipeline_viewer_event(&viewer_id, Some(&viewer_id)));
    assert!(!is_pipeline_viewer_event(
        &MenuId::new("help"),
        Some(&viewer_id)
    ));
    assert!(!is_pipeline_viewer_event(&viewer_id, None));
}

#[test]
fn pipeline_viewer_menu_text_matches_label() {
    assert_eq!(PIPELINE_VIEWER_MENU_TEXT, "Pipeline Viewer");
}
