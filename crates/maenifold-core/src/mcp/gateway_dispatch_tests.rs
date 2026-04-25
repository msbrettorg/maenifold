use super::{DispatchResult, dispatch_facade_call};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn server_with_tools(tools: Vec<Tool>) -> ConsolidatedServerTool {
    ConsolidatedServerTool {
        server_name: "Gateway Server".to_string(),
        tool_catalog: tools.clone(),
        facade_tool: Tool::new(
            "gateway_server".to_string(),
            "Gateway facade".to_string(),
            Arc::new(Map::new()),
        ),
    }
}

fn tool(name: &str, description: &str, schema: Value) -> Tool {
    let schema = schema.as_object().cloned().unwrap_or_default();
    Tool::new(
        name.to_string(),
        description.to_string(),
        Arc::new(schema),
    )
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
    let result = dispatch_facade_call(
        &sample_server(),
        None,
        Some("delete_user"),
        true,
        json!({}),
    )
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
fn unambiguous_intent_dispatches_matching_tool() {
    let params = json!({"userId":"123"});
    let result = dispatch_facade_call(
        &sample_server(),
        Some("delete existing user"),
        None,
        false,
        params.clone(),
    )
    .expect("intent dispatch should succeed");

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
fn ambiguous_intent_returns_candidates() {
    let result = dispatch_facade_call(
        &sample_server(),
        Some("search"),
        None,
        false,
        json!({}),
    )
    .expect("ambiguous intent should return candidates");

    assert_eq!(
        result,
        DispatchResult::Candidates(vec![
            ("search_projects".to_string(), "Search projects by name".to_string()),
            ("search_users".to_string(), "Search user profiles".to_string()),
        ])
    );
}

#[test]
fn zero_match_intent_suggests_learn_mode() {
    let error = dispatch_facade_call(
        &sample_server(),
        Some("rotate invoice"),
        None,
        false,
        json!({}),
    )
    .expect_err("zero-match intent should fail");

    assert!(error.to_string().contains("learn=true"));
}

#[test]
fn overly_broad_intent_returns_error() {
    let server = server_with_tools(vec![
        tool("list_users", "List users", json!({"type":"object"})),
        tool("list_roles", "List roles", json!({"type":"object"})),
        tool("list_groups", "List groups", json!({"type":"object"})),
        tool("list_orgs", "List orgs", json!({"type":"object"})),
        tool("list_teams", "List teams", json!({"type":"object"})),
        tool("list_repos", "List repos", json!({"type":"object"})),
    ]);
    let error = dispatch_facade_call(&server, Some("list"), None, false, json!({}))
        .expect_err("broad intent should fail");

    assert!(error.to_string().contains("learn=true"));
}

#[test]
fn empty_args_return_error() {
    let error = dispatch_facade_call(&sample_server(), None, None, false, json!({}))
        .expect_err("empty args should fail");

    assert!(error.to_string().contains("learn=true"));
    assert!(error.to_string().contains("intent"));
}
