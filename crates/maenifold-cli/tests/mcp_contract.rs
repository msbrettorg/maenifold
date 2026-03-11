// T-10.18: RTM NFR-3
//! MCP contract metadata tests split from `mcp_tests.rs`.

mod mcp_test_support;

use maenifold_core::ToolRegistry;
use mcp_test_support::{McpTestHarness, isolated_root};
use serde_json::json;

#[test]
fn mcp_unknown_tool_returns_error() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    h.initialize();

    let resp = h.request(
        "tools/call",
        json!({
            "name": "CompletelyBogusToolThatDoesNotExist",
            "arguments": {}
        }),
    );

    assert_eq!(resp["jsonrpc"], "2.0");

    let result = &resp["result"];
    assert_eq!(
        result["isError"], true,
        "unknown tool must set isError=true in the result"
    );

    let content = result["content"]
        .as_array()
        .expect("result.content must be an array");
    assert!(
        !content.is_empty(),
        "error result must contain at least one content entry"
    );
}

#[test]
fn mcp_server_instructions_present_and_under_500_words() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    let resp = h.initialize();

    let instructions = resp["result"]["instructions"]
        .as_str()
        .expect("instructions must be a non-null string in InitializeResult");
    assert!(!instructions.is_empty(), "instructions must not be empty");

    let word_count = instructions.split_whitespace().count();
    assert!(
        word_count < 500,
        "instructions must be fewer than 500 words (got {word_count})"
    );
}

#[test]
fn mcp_tool_names_match_registry_canonical_names() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    h.initialize();

    let resp = h.request("tools/list", json!({}));
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("result.tools array");

    let mut mcp_names: Vec<String> = tools
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("tool name must be a string")
                .to_string()
        })
        .collect();
    mcp_names.sort();

    let registry = ToolRegistry::new();
    let mut registry_names: Vec<String> = registry
        .canonical_names()
        .into_iter()
        .map(String::from)
        .collect();
    registry_names.sort();

    assert_eq!(
        mcp_names, registry_names,
        "MCP tools/list names must exactly match ToolRegistry canonical names"
    );
}
