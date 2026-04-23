#![allow(unused_imports, dead_code)]

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde_json::{Value, json};
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;

use super::executor::execute_pipeline;
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode, RetryPolicy,
};

pub(super) fn pending_node_state(session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status: NodeStatus::Pending,
        attempts: 0,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

pub(super) fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

pub(super) fn write_mcp_server_config(context: &crate::tools::ToolContext, mcp_servers: Value) {
    fs::write(
        context.config.maenifold_root.join("maenifold.json"),
        serde_json::to_vec_pretty(&json!({ "mcpServers": mcp_servers }))
            .expect("serialize MCP server config"),
    )
    .expect("write MCP server config");
}

pub(super) fn write_sleep_mcp_server_config(context: &crate::tools::ToolContext) {
    write_mcp_server_config(
        context,
        json!([{
            "type": "stdio",
            "name": "sleepmcp",
            "command": "/bin/sleep",
            "args": ["5"]
        }]),
    );
}

// NOTE: flaky retry integration test removed — requires python3 MCP server
// fixture that's environment-dependent. The retry unit tests in
// executor_retry_tests.rs cover the logic; this integration path needs
// a stable test-harness binary (not an inline Python script).

#[allow(dead_code)]
pub(super) fn write_flaky_retry_mcp_server_config(
    context: &crate::tools::ToolContext,
    counter_file: &std::path::Path,
) {
    let script = r#"
import json
import os
import sys

COUNTER_PATH = sys.argv[1] if len(sys.argv) > 1 else ""

def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "":
            return None
        line = line.strip()
        if not line:
            continue
        return json.loads(line)

def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":"), ensure_ascii=False) + "\n")
    sys.stdout.flush()

def read_counter():
    if not COUNTER_PATH:
        return 0
    try:
        with open(COUNTER_PATH, "r", encoding="utf-8") as file:
            return int((file.read() or "0").strip())
    except Exception:
        return 0

def write_counter(value):
    if not COUNTER_PATH:
        return
    with open(COUNTER_PATH, "w", encoding="utf-8") as file:
        file.write(str(value))

def text_result(text):
    return {
        "content": [{"type": "text", "text": text}],
        "isError": False
    }

TOOLS = [
    {"name": "route", "description": "routes conditional edge", "inputSchema": {"type": "object"}},
    {"name": "flaky", "description": "fails first call then succeeds", "inputSchema": {"type": "object"}},
    {"name": "sink", "description": "terminal sink", "inputSchema": {"type": "object"}}
]

while True:
    message = read_message()
    if message is None:
        break

    method = message.get("method")
    msg_id = message.get("id")
    params = message.get("params") or {}

    if method == "initialize":
        if msg_id is not None:
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "flakymcp", "version": "0.0.1"}
                }
            })
        continue

    if method == "notifications/initialized":
        continue

    if method == "tools/list":
        if msg_id is not None:
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": {"tools": TOOLS}
            })
        continue

    if method == "tools/call":
        tool_name = params.get("name")
        if tool_name == "route":
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": text_result("{\"route\":\"run\"}")
            })
            continue

        if tool_name == "sink":
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": text_result("{\"done\":true}")
            })
            continue

        if tool_name == "flaky":
            current = read_counter() + 1
            write_counter(current)
            if current == 1:
                write_message({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "error": {
                        "code": -32001,
                        "message": "transient first-call failure"
                    }
                })
            else:
                write_message({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": text_result("{\"ok\":true}")
                })
            continue

        write_message({
            "jsonrpc": "2.0",
            "id": msg_id,
            "error": {"code": -32601, "message": "unknown tool"}
        })
        continue

    if msg_id is not None:
        write_message({
            "jsonrpc": "2.0",
            "id": msg_id,
            "error": {"code": -32601, "message": "unknown method"}
        })
"#;

    write_mcp_server_config(
        context,
        json!([{
            "type": "stdio",
            "name": "flakymcp",
            "command": "/usr/bin/python3",
            "args": ["-u", "-c", script, counter_file]
        }]),
    );
}
