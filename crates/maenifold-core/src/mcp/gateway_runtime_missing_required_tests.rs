use super::*;
use crate::acp_config::IntentResolverSettings;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::StdioPool;
use serde_json::json;
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
