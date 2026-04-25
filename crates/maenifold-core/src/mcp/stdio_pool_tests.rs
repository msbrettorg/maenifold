use super::{ManagedServer, StdioPool, StdioServerConfig, collect_gateway_tools};
use crate::error::MaenifoldError;
use crate::mcp::stdio_session::StdioSession;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const PYTHON: &str = "/usr/bin/python3";
const GOOD_SERVER: &str = r#"
import json, sys
tool_name = sys.argv[1]
def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "": return None
        line = line.strip()
        if line: return json.loads(line)
def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()
def text_result(text):
    return {"content": [{"type": "text", "text": text}], "isError": False}
while True:
    message = read_message()
    if message is None: break
    method = message.get("method")
    msg_id = message.get("id")
    if method == "initialize":
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"test","version":"0.0.1"}}})
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"tools":[{"name":tool_name,"description":"desc","inputSchema":{"type":"object"}}]}})
    elif method == "tools/call":
        write_message({"jsonrpc":"2.0","id":msg_id,"result":text_result("ok")})
    else:
        write_message({"jsonrpc":"2.0","id":msg_id,"error":{"code":-32601,"message":"unknown method"}})
"#;

fn server_config(name: &str, tool_name: &str) -> StdioServerConfig {
    StdioServerConfig {
        name: name.to_string(),
        command: PYTHON.to_string(),
        args: vec!["-u".to_string(), "-c".to_string(), GOOD_SERVER.to_string(), tool_name.to_string()],
        env: HashMap::new(),
        timeout: Some(Duration::from_secs(2)),
    }
}

async fn started_server(name: &str, tool_name: &str) -> ManagedServer {
    ManagedServer {
        config: server_config(name, tool_name),
        session: Arc::new(
            StdioSession::start(
                &agent_client_protocol_schema::McpServerStdio::new(name.to_string(), PYTHON.to_string())
                    .args(vec!["-u".to_string(), "-c".to_string(), GOOD_SERVER.to_string(), tool_name.to_string()]),
                Duration::from_secs(2),
            )
            .await
            .expect("session should start"),
        ),
    }
}

#[tokio::test]
async fn stdio_pool_duplicate_server_name_rejected() {
    let error = match StdioPool::start(vec![server_config("dup", "tool-a"), server_config("dup", "tool-b")]).await {
        Ok(_) => panic!("duplicate server names should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.to_string(),
        "Duplicate gateway server name: 'dup'. Each server must have a unique name."
    );
}

#[tokio::test]
async fn stdio_pool_build_rejects_tool_collisions() {
    let left = started_server("server-a", "shared-tool").await;
    let right = started_server("server-b", "shared-tool").await;
    let mut servers = HashMap::new();
    servers.insert("server-a".to_string(), left);
    servers.insert("server-b".to_string(), right);

    let error = collect_gateway_tools(&servers).expect_err("tool collisions should fail");
    let rendered = error.to_string();
    assert!(rendered.contains("MCP tool name collision: 'shared-tool'"));
    assert!(rendered.contains("server-a"));
    assert!(rendered.contains("server-b"));

    for server in servers.into_values() {
        server.session.shutdown().await;
    }
}

#[tokio::test]
async fn stdio_pool_partial_startup_keeps_survivors_usable() {
    let mut bad = server_config("broken", "unused");
    bad.args = vec!["-u".to_string(), "-c".to_string(), "import sys; sys.exit(1)".to_string()];
    let pool = StdioPool::start(vec![server_config("good", "survivor"), bad])
        .await
        .expect("pool should start with surviving servers");

    assert_eq!(
        pool.all_gateway_tools()
            .into_iter()
            .map(|(server, tools)| (server, tools.into_iter().map(|tool| tool.name.to_string()).collect::<Vec<_>>() ))
            .collect::<Vec<_>>(),
        vec![("good".to_string(), vec!["survivor".to_string()])]
    );
    assert_eq!(
        pool.all_tools().into_iter().map(|tool| tool.name.to_string()).collect::<Vec<_>>(),
        vec!["survivor".to_string()]
    );
    pool.call_tool("good", "survivor", json!({}))
        .await
        .expect("surviving server should remain usable");
    match pool.call_tool("broken", "survivor", json!({})).await.expect_err("failed server should not be routed") {
        MaenifoldError::JsonRpc { message, .. } => assert_eq!(message, "Gateway: unknown server 'broken'"),
        other => panic!("unexpected error: {other}"),
    }
    pool.shutdown().await;
}
