use super::config_tools::{handle_tool_call, mutation_state};
use crate::acp_config::AgentsConfig;
use crate::config::Config;
use serde_json::{Value, json};

#[tokio::test]
async fn mcp_servers_schema_config_tools_add_and_delete_object_keyed_entries_order_stable() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        r#"{
            "mcpServers": {
                "alpha": {
                    "command": "/usr/bin/env",
                    "args": ["alpha"],
                    "gateway": true
                }
            },
            "agents": {}
        }"#,
    )
    .expect("write object-keyed config");
    let config = test_config(root.path());
    let state = mutation_state(
        &config,
        crate::api::config_manager::ConfigManager::new(&config),
    );

    let added = handle_tool_call(
        state.clone(),
        "AddMcpServer",
        json!({
            "name": "beta",
            "entry": {
                "url": "https://example.invalid/mcp",
                "gateway": true
            }
        }),
    )
    .await
    .expect("adding object-keyed server should succeed");
    let added = first_text_json(added).expect("add response should be JSON");
    let servers = added
        .get("mcpServers")
        .and_then(Value::as_object)
        .expect("mcpServers should be an object");
    assert_eq!(
        servers.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
    assert_eq!(
        servers
            .get("beta")
            .and_then(|entry| entry.get("url"))
            .and_then(Value::as_str),
        Some("https://example.invalid/mcp")
    );

    let removed = handle_tool_call(state, "DeleteMcpServer", json!({ "name": "alpha" }))
        .await
        .expect("removing object-keyed server should succeed");
    let removed = first_text_json(removed).expect("delete response should be JSON");
    let servers = removed
        .get("mcpServers")
        .and_then(Value::as_object)
        .expect("mcpServers should be an object");
    assert_eq!(
        servers.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["beta"]
    );
}

fn first_text_json(result: rmcp::model::CallToolResult) -> Result<Value, serde_json::Error> {
    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|content| content.text.as_ref())
        .expect("text content");
    serde_json::from_str(text)
}

fn test_config(root: &std::path::Path) -> Config {
    Config {
        maenifold_root: root.to_path_buf(),
        memory_path: root.join("memory"),
        assets_path: root.join("assets"),
        thinking_path: root.join("memory").join("thinking"),
        sequential_path: root.join("memory").join("thinking").join("sequential"),
        database_path: root.join("memory.db"),
        mcp_allowed_hosts: Vec::new(),
        mcp_allowed_origins: Vec::new(),
        settings: AgentsConfig::load(root)
            .expect("test config should load")
            .settings
            .unwrap_or_default(),
    }
}
