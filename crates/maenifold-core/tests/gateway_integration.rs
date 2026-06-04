mod common;

use maenifold_core::error::MaenifoldError;
use maenifold_core::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;

const TEST_SERVER: &str = r#"
import json, sys
server_name = sys.argv[1]
tool_specs = {spec["name"]: spec for spec in json.loads(sys.argv[2])}

def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "":
            return None
        line = line.strip()
        if line:
            return json.loads(line)

def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()

while True:
    message = read_message()
    if message is None:
        break
    method = message.get("method")
    message_id = message.get("id")
    if method == "initialize":
        write_message({"jsonrpc":"2.0","id":message_id,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":server_name,"version":"0.0.1"}}})
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        tools = [{"name": spec["name"], "description": spec.get("description", spec["name"]), "inputSchema": {"type":"object","properties":{}}} for spec in tool_specs.values()]
        write_message({"jsonrpc":"2.0","id":message_id,"result":{"tools":tools}})
    elif method == "tools/call":
        tool_name = message.get("params", {}).get("name")
        spec = tool_specs.get(tool_name)
        if spec is None:
            write_message({"jsonrpc":"2.0","id":message_id,"error":{"code":-32601,"message":f"Unknown tool: {tool_name}"}})
        elif "error" in spec:
            error = spec["error"]
            write_message({"jsonrpc":"2.0","id":message_id,"error":{"code":error["code"],"message":error["message"],"data":error.get("data")}})
        else:
            write_message({"jsonrpc":"2.0","id":message_id,"result":{"content":[{"type":"text","text":spec["text"]}],"isError":False}})
    else:
        write_message({"jsonrpc":"2.0","id":message_id,"error":{"code":-32601,"message":f"Unknown method: {method}"}})
"#;

fn ok_tool(name: &str, text: &str) -> Value {
    json!({
        "name": name,
        "description": format!("{name} description"),
        "text": text,
    })
}

fn error_tool(name: &str, code: i64, message: &str, data: Value) -> Value {
    json!({
        "name": name,
        "description": format!("{name} description"),
        "error": {
            "code": code,
            "message": message,
            "data": data,
        },
    })
}

fn server_config(name: &str, tool_specs: Vec<Value>) -> StdioServerConfig {
    StdioServerConfig {
        name: name.to_string(),
        command: common::locate_python3(),
        args: vec![
            "-u".to_string(),
            "-c".to_string(),
            TEST_SERVER.to_string(),
            name.to_string(),
            serde_json::to_string(&tool_specs).expect("tool specs should serialize"),
        ],
        env: HashMap::new(),
        timeout: Some(Duration::from_secs(2)),
    }
}

fn call_text(result: &CallToolResult) -> String {
    serde_json::to_value(result).expect("call result should serialize")["content"][0]["text"]
        .as_str()
        .expect("call result should contain text")
        .to_string()
}

#[tokio::test]
async fn gateway_routes_calls_to_the_correct_upstream() {
    let pool = StdioPool::start(
        vec![
            server_config("alpha", vec![ok_tool("alpha_echo", "handled by alpha")]),
            server_config("beta", vec![ok_tool("beta_echo", "handled by beta")]),
        ],
        std::sync::Arc::new(maenifold_core::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("pool should start");

    let alpha = pool
        .call_tool("alpha", "alpha_echo", json!({"message":"hello"}))
        .await
        .expect("alpha call should succeed");
    let beta = pool
        .call_tool("beta", "beta_echo", json!({"message":"hello"}))
        .await
        .expect("beta call should succeed");

    assert_eq!(call_text(&alpha), "handled by alpha");
    assert_eq!(call_text(&beta), "handled by beta");
    pool.shutdown().await;
}

#[tokio::test]
async fn gateway_propagates_upstream_json_rpc_errors() {
    let pool = StdioPool::start(
        vec![server_config(
            "gamma",
            vec![error_tool(
                "explode",
                -32042,
                "upstream failure",
                json!({"detail":"boom"}),
            )],
        )],
        std::sync::Arc::new(maenifold_core::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("pool should start");

    let error = pool
        .call_tool("gamma", "explode", json!({"input":"value"}))
        .await
        .expect_err("upstream error should propagate");

    match error {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => {
            assert_eq!(code, -32042);
            assert_eq!(message, "upstream failure");
            assert_eq!(data, json!({"detail":"boom"}));
        }
        other => panic!("unexpected error: {other}"),
    }
    pool.shutdown().await;
}

#[tokio::test]
async fn gateway_returns_tool_not_found_for_unknown_upstream_tool() {
    let pool = StdioPool::start(
        vec![server_config(
            "delta",
            vec![ok_tool("known_tool", "available")],
        )],
        std::sync::Arc::new(maenifold_core::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("pool should start");

    let error = pool
        .call_tool("delta", "missing_tool", json!({}))
        .await
        .expect_err("missing tool should fail");

    match error {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => {
            assert_eq!(code, -32601);
            assert_eq!(message, "Unknown tool: missing_tool");
            assert_eq!(data, Value::Null);
        }
        other => panic!("unexpected error: {other}"),
    }
    pool.shutdown().await;
}
