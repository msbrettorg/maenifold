use super::{
    ManagedServer, ServerStartupStatus, StdioPool, StdioServerConfig, collect_gateway_tools,
};
use crate::mcp::stdio_session::StdioSession;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

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
const COUNTING_SERVER: &str = r#"
import json, os, sys, time
count_dir = os.environ["COUNT_DIR"]
open(os.path.join(count_dir, f"spawn-{os.getpid()}-{time.time_ns()}"), "w").close()
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
        args: vec![
            "-u".to_string(),
            "-c".to_string(),
            GOOD_SERVER.to_string(),
            tool_name.to_string(),
        ],
        env: HashMap::new(),
        timeout: Some(Duration::from_secs(2)),
    }
}

async fn started_server(name: &str, tool_name: &str) -> ManagedServer {
    ManagedServer::running(
        server_config(name, tool_name),
        StdioSession::start(
            &agent_client_protocol_schema::McpServerStdio::new(
                name.to_string(),
                PYTHON.to_string(),
            )
            .args(vec![
                "-u".to_string(),
                "-c".to_string(),
                GOOD_SERVER.to_string(),
                tool_name.to_string(),
            ]),
            Duration::from_secs(2),
        )
        .await
        .expect("session should start"),
    )
}

#[tokio::test]
async fn stdio_pool_duplicate_server_name_rejected() {
    let error = match StdioPool::start(vec![
        server_config("dup", "tool-a"),
        server_config("dup", "tool-b"),
    ], std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()))
    .await
    {
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
        if let Some(session) = server.session {
            session.shutdown().await;
        }
    }
}

#[tokio::test]
async fn stdio_pool_partial_startup_keeps_survivors_usable() {
    let mut bad = server_config("broken", "unused");
    bad.args = vec![
        "-u".to_string(),
        "-c".to_string(),
        "import sys; print('startup stderr line', file=sys.stderr); sys.exit(1)".to_string(),
    ];
    let pool = StdioPool::start(vec![server_config("good", "survivor"), bad], std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()))
        .await
        .expect("pool should start with surviving servers");

    assert_eq!(
        pool.all_gateway_tools()
            .into_iter()
            .map(|(server, tools)| (
                server,
                tools
                    .into_iter()
                    .map(|tool| tool.name.to_string())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![("good".to_string(), vec!["survivor".to_string()])]
    );
    assert_eq!(
        pool.all_tools()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>(),
        vec!["survivor".to_string()]
    );
    pool.call_tool("good", "survivor", json!({}))
        .await
        .expect("surviving server should remain usable");
    let health = pool.health().await;
    let broken = health
        .servers
        .iter()
        .find(|server| server.name == "broken")
        .expect("broken server should be visible in health");
    assert!(broken.configured);
    assert!(broken.enabled);
    assert!(broken.degraded);
    assert!(!broken.running);
    assert_eq!(broken.status, ServerStartupStatus::Failed);
    assert_eq!(broken.tool_count, 0);
    assert_eq!(broken.stderr_tail, vec!["startup stderr line".to_string()]);
    assert!(
        broken
            .last_start_error
            .as_deref()
            .unwrap_or("")
            .contains("connection")
    );

    let error = pool
        .call_tool("broken", "survivor", json!({}))
        .await
        .expect_err("failed server should report degraded restart state");
    assert!(error.to_string().contains("restart backoff active"));
    pool.shutdown().await;
}

#[tokio::test]
async fn stdio_pool_dead_server_restart_is_single_flight() {
    let dir = tempfile::tempdir().expect("tempdir should be created");
    let mut config = server_config("counting", "single-flight");
    config.args = vec![
        "-u".to_string(),
        "-c".to_string(),
        COUNTING_SERVER.to_string(),
        "single-flight".to_string(),
    ];
    config.env.insert(
        "COUNT_DIR".to_string(),
        dir.path().to_string_lossy().to_string(),
    );
    config
        .env
        .insert("SECRET_TOKEN".to_string(), "redacted-value".to_string());
    let pool = Arc::new(
        StdioPool::start(vec![config], std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()))
            .await
            .expect("pool should start"),
    );

    let original = {
        let servers = pool.servers.read().await;
        servers
            .get("counting")
            .and_then(|server| server.session.clone())
            .expect("session should exist")
    };
    original.shutdown().await;

    let mut calls = JoinSet::new();
    for _ in 0..16 {
        let pool = Arc::clone(&pool);
        calls.spawn(async move { pool.call_tool("counting", "single-flight", json!({})).await });
    }
    while let Some(result) = calls.join_next().await {
        result
            .expect("task should join")
            .expect("tool call should succeed after shared restart");
    }

    let spawn_count = std::fs::read_dir(dir.path())
        .expect("spawn dir should be readable")
        .count();
    assert_eq!(spawn_count, 2);
    let health = pool.health().await;
    let server = health
        .servers
        .iter()
        .find(|server| server.name == "counting")
        .expect("counting health should exist");
    assert!(server.running);
    assert_eq!(server.status, ServerStartupStatus::Started);
    assert_eq!(
        server.env_keys,
        vec!["COUNT_DIR".to_string(), "SECRET_TOKEN".to_string()]
    );
    assert!(server.last_start_error.is_none());
    pool.shutdown().await;
}
