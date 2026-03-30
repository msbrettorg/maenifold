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
