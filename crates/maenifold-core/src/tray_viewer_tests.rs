use super::{
    PIPELINE_VIEWER_MENU_TEXT, is_pipeline_viewer_event, pipeline_viewer_html_for_payload,
    pipeline_viewer_resource_uri, select_pipeline_session_id,
};
use crate::mcp::tool_utils::PIPELINE_UI_RESOURCE_URI;
use serde_json::json;
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
fn pipeline_viewer_targets_pipeline_mcp_app_resource() {
    assert_eq!(PIPELINE_VIEWER_MENU_TEXT, "Pipeline Viewer");
    assert_eq!(pipeline_viewer_resource_uri(), PIPELINE_UI_RESOURCE_URI);
}

#[test]
fn pipeline_viewer_html_embeds_view_payload_from_mcp_resource() {
    let html = pipeline_viewer_html_for_payload(&json!({
        "pipelineId": "visual-uat",
        "status": "active",
        "ready": ["next"],
        "active": ["now"],
        "completed": ["done"],
        "nodes": [
            { "id": "now", "type": "tool", "status": "active" },
            { "id": "next", "type": "workflow", "status": "ready" }
        ],
        "edges": [{ "from": "now", "to": "next", "type": "success" }]
    }))
    .expect("Pipeline viewer HTML should embed sample payload");

    assert!(html.contains("window.renderPipeline"));
    assert!(html.contains("visual-uat"));
    assert!(html.contains(r#"aria-label="Close Pipeline viewer""#));
    assert!(html.contains("window.ipc.postMessage"));
    assert!(html.contains("close-pipeline-viewer"));
    assert!(html.contains(r#"<script type="application/json" id="pipeline-data">{"#));
}

#[test]
fn pipeline_viewer_selects_active_pipeline_before_completed() {
    let payload = json!({
        "sessions": [
            { "pipelineId": "pipeline-999", "status": "completed" },
            { "pipelineId": "pipeline-100", "status": "active" },
            { "pipelineId": "pipeline-101", "status": "pending" }
        ]
    });

    assert_eq!(
        select_pipeline_session_id(&payload).as_deref(),
        Some("pipeline-101")
    );
}

#[test]
fn pipeline_viewer_falls_back_to_pipeline_list_shape() {
    let payload = json!({
        "pipelines": [
            { "id": "pipeline-100", "status": "completed" },
            { "sessionId": "pipeline-101", "status": "completed" }
        ]
    });

    assert_eq!(
        select_pipeline_session_id(&payload).as_deref(),
        Some("pipeline-101")
    );
}
