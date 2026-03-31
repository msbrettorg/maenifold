use super::*;
use std::collections::BTreeSet;

// T-5.2: RTM FR-22.4 — all registered tools are represented in the MCP schema mapping.
#[test]
fn t_5_2_registry_mapping_covers_all_tools() {
    let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
    let names = tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<BTreeSet<_>>();
    let expected_tool_count = ToolRegistry::new().canonical_names().len();

    assert_eq!(tools.len(), expected_tool_count);
    assert_eq!(names.len(), expected_tool_count);
    assert!(names.contains("WriteMemory"));
    assert!(names.contains("ListAgents"));
    assert!(names.contains("ReadMcpResource"));
    assert!(names.contains("RunFullBenchmark"));
}

// T-5.2: RTM FR-22.4 — WriteMemory schema declares required fields.
#[test]
fn t_5_2_write_memory_schema_requires_title_and_content() {
    let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
    let write_memory = tools
        .into_iter()
        .find(|tool| tool.name == "WriteMemory")
        .expect("WriteMemory tool should exist");

    let required = write_memory
        .input_schema
        .get("required")
        .and_then(Value::as_array)
        .expect("required fields should be present")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();

    assert_eq!(required, vec!["title", "content"]);
}

// T-5.2: RTM FR-22.4 — canonical names survive the McpToolDefinition mapping.
#[test]
fn t_5_2_registry_names_match_canonical_names() {
    let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
    let names = tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>();

    assert!(names.contains(&"WriteMemory".to_string()));
    assert!(names.contains(&"ListAgents".to_string()));
    assert!(names.contains(&"ReadMcpResource".to_string()));
    assert!(names.contains(&"RunFullBenchmark".to_string()));
}

// T-5.1: RTM FR-22.1 — handler builds and exposes correct server info.
#[test]
fn t_5_1_handler_server_info_matches_expected() {
    let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler should build");
    let info = handler.get_info();

    assert_eq!(info.server_info.name, "maenifold");
    assert!(!info.server_info.version.is_empty());
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert_eq!(
        info.capabilities
            .resources
            .as_ref()
            .and_then(|resources| resources.list_changed),
        Some(true)
    );
    assert!(info.instructions.is_some());
    assert!(
        info.instructions
            .as_ref()
            .unwrap()
            .contains("maenifold tools")
    );
}

// T-5.1: RTM FR-22.1 — handler converts all registered McpToolDefinitions into rmcp Tools.
#[test]
fn t_5_1_handler_builds_all_rmcp_tools() {
    let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler should build");
    let expected_tool_count = ToolRegistry::new().canonical_names().len();
    assert_eq!(handler.tools.len(), expected_tool_count);

    let names: BTreeSet<&str> = handler.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains("ListAgents"));
    assert!(names.contains("WriteMemory"));
    assert!(names.contains("SequentialThinking"));
    assert!(names.contains("Pipeline"));
    assert!(names.contains("ReadMcpResource"));
}

#[test]
fn t_27_4_http_handler_does_not_advertise_resource_list_changed() {
    let handler = MaenifoldHandler::new(TransportKind::Http).expect("handler should build");
    let info = handler.get_info();

    assert!(info.capabilities.resources.is_some());
    assert_eq!(
        info.capabilities
            .resources
            .as_ref()
            .and_then(|resources| resources.list_changed),
        None
    );
}

#[test]
fn t_27_5_message_errors_are_not_validation_errors() {
    let err = MaenifoldError::Message("database unavailable".to_string());

    assert!(!is_validation_error(&err));
    assert_eq!(
        format_tool_error("WriteMemory", &err),
        "Error invoking WriteMemory: database unavailable"
    );
}

#[test]
fn t_27_5_message_resource_errors_map_to_internal_error() {
    let err = self::resources::map_resource_error(MaenifoldError::Message(
        "database unavailable".to_string(),
    ));

    assert_eq!(err.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
    assert_eq!(err.message.as_ref(), "database unavailable");
}

// T-74.2: stale Mcp-Session-Id triggers JSON 404 with re-init instructions.
#[tokio::test]
async fn t_74_2_stale_session_returns_json_404() {
    use axum::body::Body;
    use axum::http::StatusCode;

    let response = axum::http::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found: Session not found"))
        .unwrap();

    let result =
        http::rewrite_stale_session_response(Some("expired-session-abc".to_string()), response)
            .await;

    assert_eq!(result.status(), StatusCode::NOT_FOUND);

    let ct = result
        .headers()
        .get("content-type")
        .expect("should have Content-Type")
        .to_str()
        .unwrap();
    assert!(
        ct.contains("application/json"),
        "expected JSON content-type: {ct}"
    );

    let bytes = axum::body::to_bytes(result.into_body(), 4096)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(
        body.contains("session_expired"),
        "body missing error code: {body}"
    );
    assert!(
        body.contains("initialize"),
        "body should mention re-initialization: {body}"
    );
}

// T-74.2: 200 OK responses are not rewritten even when a session ID is present.
#[tokio::test]
async fn t_74_2_ok_response_not_rewritten() {
    use axum::body::Body;
    use axum::http::StatusCode;

    let response = axum::http::Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("ok"))
        .unwrap();

    let result =
        http::rewrite_stale_session_response(Some("active-session".to_string()), response).await;

    assert_eq!(result.status(), StatusCode::OK);
}

// T-74.2: requests without Mcp-Session-Id pass 404s through unchanged.
#[tokio::test]
async fn t_74_2_no_session_id_passes_through() {
    use axum::body::Body;
    use axum::http::StatusCode;

    let response = axum::http::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found: Session not found"))
        .unwrap();

    let result = http::rewrite_stale_session_response(None, response).await;

    assert_eq!(result.status(), StatusCode::NOT_FOUND);
    // Body should be the original plain text, not our JSON rewrite
    let bytes = axum::body::to_bytes(result.into_body(), 4096)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(
        !body.contains("session_expired"),
        "should not rewrite when no session ID was sent: {body}"
    );
}

// T-74.2: unrelated 404s must pass through even when a session ID header is present.
#[tokio::test]
async fn t_74_2_unrelated_404_not_rewritten() {
    use axum::body::Body;
    use axum::http::StatusCode;

    let response = axum::http::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Route missing"))
        .unwrap();

    let result =
        http::rewrite_stale_session_response(Some("active-session".to_string()), response).await;

    assert_eq!(result.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(result.into_body(), 4096)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert_eq!(body, "Route missing");
}
