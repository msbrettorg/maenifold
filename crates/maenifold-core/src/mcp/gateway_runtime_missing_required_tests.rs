use super::*;
use crate::acp_config::IntentResolverSettings;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::StdioPool;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

async fn state() -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("empty pool"),
    );
    let registry = GatewayRegistry::build(Vec::new(), &BTreeMap::new()).expect("registry");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(
        pool,
        resources,
        registry,
        IntentResolverSettings::default(),
        PathBuf::from("."),
    )
}

fn text(result: &CallToolResult) -> String {
    result.content[0]
        .raw
        .as_text()
        .expect("text content")
        .text
        .clone()
}

fn json_text(result: &CallToolResult) -> Value {
    serde_json::from_str(&text(result)).expect("json result")
}

#[tokio::test]
async fn gateway_execute_lists_all_missing_required_native_parameters() {
    let state = state().await;

    let err = state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": {}
        }))
        .await
        .expect_err("missing required parameters should fail");

    let message = err.to_string();
    assert!(
        message.contains("Missing required parameter(s)"),
        "{message}"
    );
    assert!(
        message.contains("maenifold.memory/WriteMemory"),
        "{message}"
    );
    assert!(message.contains("content"), "{message}");
    assert!(message.contains("title"), "{message}");
    assert!(
        message.contains("GetHelp toolName=\"WriteMemory\""),
        "{message}"
    );
    assert!(!message.contains("uri_or_path"), "{message}");
}

#[tokio::test]
async fn gateway_execute_missing_required_message_excludes_non_required_errors() {
    let state = state().await;

    let err = state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": { "title": 7 }
        }))
        .await
        .expect_err("mixed missing/type errors should fail");

    let message = err.to_string();
    assert!(
        message.contains("Missing required parameter(s)"),
        "{message}"
    );
    assert!(message.contains("content"), "{message}");
    assert!(!message.contains("title, content"), "{message}");
    assert!(!message.contains("type"), "{message}");
}

#[tokio::test]
async fn gateway_index_default_catalog_omits_commands_from_domain_entries() {
    let state = state().await;

    let result = state
        .handle_tool_call("GatewayIndex", json!({}), None)
        .await
        .expect("default GatewayIndex should succeed")
        .expect("GatewayIndex should return a result");
    let value = json_text(&result);
    let domains = value["domains"].as_array().expect("domains array");
    let memory_domain = domains
        .iter()
        .find(|domain| domain["name"] == "maenifold.memory")
        .expect("maenifold.memory domain");

    assert!(
        memory_domain.get("commands").is_none(),
        "{memory_domain:#?}"
    );
}

#[tokio::test]
async fn gateway_index_include_commands_embeds_command_summaries_without_schemas() {
    let state = state().await;

    let result = state
        .handle_tool_call("GatewayIndex", json!({ "includeCommands": true }), None)
        .await
        .expect("GatewayIndex includeCommands should succeed")
        .expect("GatewayIndex should return a result");
    let value = json_text(&result);
    let domains = value["domains"].as_array().expect("domains array");
    let memory_domain = domains
        .iter()
        .find(|domain| domain["name"] == "maenifold.memory")
        .expect("maenifold.memory domain");
    let commands = memory_domain["commands"]
        .as_array()
        .expect("commands array");
    let read_memory = commands
        .iter()
        .find(|command| command["name"] == "ReadMemory")
        .expect("ReadMemory command");

    assert!(
        read_memory["description"]
            .as_str()
            .is_some_and(|description| !description.is_empty()),
        "{read_memory:#?}"
    );
    assert!(read_memory.get("schema").is_none(), "{read_memory:#?}");
    assert!(
        commands
            .iter()
            .all(|command| command.get("schema").is_none())
    );
}
