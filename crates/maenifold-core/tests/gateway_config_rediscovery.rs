mod common;

use common::gateway_http::{
    TestServer, gateway_config, initialize, result_text, rpc, stdio_server,
};
use maenifold_core::Config;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn discover_mcp_tools_includes_gateway_servers_added_after_startup() {
    let server = TestServer::start(gateway_config(vec![stdio_server(
        "alpha",
        vec![json!({"name": "alpha_ping", "description": "alpha ping", "text": "alpha"})],
    )]))
    .await;

    initialize(&server).await;

    let initial = discover_mcp_tools(&server).await;
    assert!(
        server_tools(&initial, "alpha")
            .iter()
            .any(|tool| tool["name"] == "alpha.alpha_ping")
    );
    assert!(find_server(&initial, "beta").is_none());

    let updated = gateway_config(vec![
        stdio_server(
            "alpha",
            vec![json!({"name": "alpha_ping", "description": "alpha ping", "text": "alpha"})],
        ),
        stdio_server(
            "beta",
            vec![json!({"name": "beta_ping", "description": "beta ping", "text": "beta"})],
        ),
    ]);
    std::fs::write(
        Config::current().maenifold_root.join("maenifold.json"),
        serde_json::to_vec_pretty(&updated).expect("updated config should serialize"),
    )
    .expect("updated config should write");

    let rediscovered = discover_mcp_tools(&server).await;
    assert!(
        server_tools(&rediscovered, "beta")
            .iter()
            .any(|tool| tool["name"] == "beta.beta_ping"),
        "DiscoverMcpTools should include tools from gateway servers added after startup"
    );

    server.stop().await;
}

async fn discover_mcp_tools(server: &TestServer) -> Value {
    let response = rpc(
        server,
        "tools/call",
        json!({"name":"DiscoverMcpTools","arguments":{}}),
        None,
    )
    .await;
    serde_json::from_str(result_text(&response)).expect("DiscoverMcpTools should return JSON")
}

fn find_server<'a>(results: &'a Value, server_name: &str) -> Option<&'a Value> {
    results
        .as_array()
        .expect("discovery results should be an array")
        .iter()
        .find(|entry| entry["server"] == server_name)
}

fn server_tools<'a>(results: &'a Value, server_name: &str) -> &'a [Value] {
    find_server(results, server_name)
        .unwrap_or_else(|| panic!("expected discovery results for server {server_name}"))
        .get("tools")
        .and_then(Value::as_array)
        .expect("server tools should be an array")
}
