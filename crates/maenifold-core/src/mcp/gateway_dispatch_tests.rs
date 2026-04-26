use super::{DispatchResult, dispatch_facade_call};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use rmcp::model::Tool;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

fn server_with_tools(tools: Vec<Tool>) -> ConsolidatedServerTool {
    ConsolidatedServerTool {
        server_name: "Gateway Server".to_string(),
        tool_catalog: tools.clone(),
        tool_validators: HashMap::new(),
    }
}

fn tool(name: &str, description: &str, schema: Value) -> Tool {
    let schema = schema.as_object().cloned().unwrap_or_default();
    Tool::new(name.to_string(), description.to_string(), Arc::new(schema))
}

fn sample_server() -> ConsolidatedServerTool {
    server_with_tools(vec![
        tool(
            "create_user",
            "Create a new user account",
            json!({"type":"object","properties":{"email":{"type":"string"}}}),
        ),
        tool(
            "delete_user",
            "Delete an existing user account",
            json!({"type":"object","properties":{"userId":{"type":"string"}}}),
        ),
        tool(
            "search_users",
            "Search user profiles",
            json!({"type":"object","properties":{"query":{"type":"string"}}}),
        ),
        tool(
            "search_projects",
            "Search projects by name",
            json!({"type":"object","properties":{"query":{"type":"string"}}}),
        ),
    ])
}

#[test]
fn learn_mode_returns_catalog() {
    let result = dispatch_facade_call(&sample_server(), None, None, true, json!({}))
        .expect("learn catalog should succeed");

    let DispatchResult::LearnCatalog(text) = result else {
        panic!("expected learn catalog");
    };
    assert!(text.contains("Available commands for Gateway Server"));
    assert!(text.contains("- create_user: Create a new user account"));
    assert!(text.contains("- delete_user: Delete an existing user account"));
}

#[test]
fn learn_mode_with_command_returns_schema() {
    let result = dispatch_facade_call(&sample_server(), None, Some("delete_user"), true, json!({}))
        .expect("learn schema should succeed");

    assert_eq!(
        result,
        DispatchResult::LearnSchema(
            json!({"type":"object","properties":{"userId":{"type":"string"}}})
        )
    );
}

#[test]
fn direct_command_dispatch_returns_upstream_target() {
    let params = json!({"userId":"123"});
    let result = dispatch_facade_call(
        &sample_server(),
        None,
        Some("delete_user"),
        false,
        params.clone(),
    )
    .expect("direct dispatch should succeed");

    assert_eq!(
        result,
        DispatchResult::CallTool {
            server_name: "Gateway Server".to_string(),
            tool_name: "delete_user".to_string(),
            parameters: params,
        }
    );
}

#[test]
fn unknown_command_returns_helpful_error() {
    let error = dispatch_facade_call(
        &sample_server(),
        None,
        Some("missing_tool"),
        false,
        json!({}),
    )
    .expect_err("unknown command should fail");

    assert!(error.to_string().contains("missing_tool"));
    assert!(error.to_string().contains("Available commands"));
}

#[test]
fn intent_dispatch_is_removed_from_facade() {
    let error = dispatch_facade_call(
        &sample_server(),
        Some("rotate invoice"),
        None,
        false,
        json!({}),
    )
    .expect_err("intent dispatch should fail");

    assert!(error.to_string().contains("GatewayIntent"));
}

#[test]
fn empty_args_return_error() {
    let error = dispatch_facade_call(&sample_server(), None, None, false, json!({}))
        .expect_err("empty args should fail");

    assert!(error.to_string().contains("learn=true"));
    assert!(error.to_string().contains("intent"));
}
