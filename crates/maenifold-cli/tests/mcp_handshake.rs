// T-10.18: RTM NFR-3
//! MCP handshake and tool invocation tests split from `mcp_tests.rs`.

mod mcp_test_support;

use mcp_test_support::{EXPECTED_TOOL_COUNT, McpTestHarness, isolated_root};
use serde_json::{Value, json};

#[test]
fn mcp_initialize_returns_server_info() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    let resp = h.initialize();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(
        resp.get("error").is_none(),
        "initialize must not return a JSON-RPC error"
    );

    let result = &resp["result"];
    let info = &result["serverInfo"];
    assert_eq!(info["name"], "maenifold", "server name mismatch");
    assert!(
        info["version"].is_string(),
        "server version must be a string"
    );
    assert!(
        result["protocolVersion"].is_string(),
        "protocolVersion must be present as a string"
    );
    assert!(
        result["capabilities"]["tools"].is_object(),
        "tools capability must be advertised"
    );
}

#[test]
fn mcp_tools_list_returns_all_28_tools_with_schemas() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    h.initialize();

    let resp = h.request("tools/list", json!({}));

    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(
        resp.get("error").is_none(),
        "tools/list must not return a JSON-RPC error"
    );

    let tools = resp["result"]["tools"]
        .as_array()
        .expect("result.tools must be an array");
    assert_eq!(
        tools.len(),
        EXPECTED_TOOL_COUNT,
        "expected {EXPECTED_TOOL_COUNT} tools, got {}",
        tools.len()
    );

    for tool in tools {
        let name = tool["name"]
            .as_str()
            .expect("every tool must have a string name");
        assert!(
            tool["description"].is_string(),
            "tool '{name}' must have a string description"
        );
        let schema = tool["inputSchema"]
            .as_object()
            .unwrap_or_else(|| panic!("tool '{name}' must have an inputSchema object"));
        assert_eq!(
            schema.get("type").and_then(Value::as_str),
            Some("object"),
            "tool '{name}' inputSchema.type must be \"object\""
        );
    }
}

#[test]
fn mcp_tools_call_dispatches_known_tool() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    h.initialize();

    let resp = h.request(
        "tools/call",
        json!({
            "name": "GetConfig",
            "arguments": {}
        }),
    );

    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(
        resp.get("error").is_none(),
        "tools/call for a known tool must not return a JSON-RPC error"
    );

    let result = &resp["result"];
    let content = result["content"]
        .as_array()
        .expect("result.content must be an array");
    assert!(!content.is_empty(), "result.content must not be empty");

    let is_error = result.get("isError").and_then(Value::as_bool);
    assert_ne!(
        is_error,
        Some(true),
        "GetConfig must not report isError=true"
    );
}
