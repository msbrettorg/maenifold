use super::StdioSession;
use crate::error::{GATEWAY_TIMEOUT_ERROR, MaenifoldError};
use crate::mcp::{PendingRequestRegistry, SamplingSessionRegistry};
use agent_client_protocol_schema::McpServerStdio;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const PYTHON: &str = "/usr/bin/python3";
const TIMEOUT_SERVER: &str = r#"
import json, sys, time
def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "": return None
        line = line.strip()
        if line: return json.loads(line)
def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()
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
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"tools":[{"name":"slow","description":"desc","inputSchema":{"type":"object"}}]}})
    elif method == "tools/call":
        time.sleep(60)
"#;
const GRANDCHILD_TIMEOUT_SERVER: &str = r#"
import json, subprocess, sys, time
marker = sys.argv[1]
def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "": return None
        line = line.strip()
        if line: return json.loads(line)
def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()
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
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"tools":[{"name":"spawn","description":"desc","inputSchema":{"type":"object"}}]}})
    elif method == "tools/call":
        subprocess.Popen([sys.executable, "-c", "import pathlib, sys, time; time.sleep(1); pathlib.Path(sys.argv[1]).write_text('alive')", marker])
        time.sleep(60)
"#;

#[tokio::test]
async fn stdio_session_timeout_poisoning_blocks_followup_calls() {
    let session = StdioSession::start(
        &McpServerStdio::new("slow-server".to_string(), PYTHON.to_string()).args(vec![
            "-u".to_string(),
            "-c".to_string(),
            TIMEOUT_SERVER.to_string(),
        ]),
        Duration::from_millis(100),
    )
    .await
    .expect("session should start");

    match session
        .call_tool("slow", json!({}))
        .await
        .expect_err("call should time out")
    {
        MaenifoldError::JsonRpc { code, .. } => assert_eq!(code, GATEWAY_TIMEOUT_ERROR),
        other => panic!("unexpected error: {other}"),
    }
    assert!(session.is_poisoned());
    match session
        .call_tool("slow", json!({}))
        .await
        .expect_err("poisoned session should reject follow-up calls")
    {
        MaenifoldError::SessionUnusable => {}
        other => panic!("unexpected error: {other}"),
    }
    session.shutdown().await;
}

#[tokio::test]
async fn stdio_session_timeout_kills_process_group_children() {
    let dir = tempfile::tempdir().expect("tempdir should be created");
    let marker = dir.path().join("grandchild-survived");
    let session = StdioSession::start(
        &McpServerStdio::new("spawn-server".to_string(), PYTHON.to_string()).args(vec![
            "-u".to_string(),
            "-c".to_string(),
            GRANDCHILD_TIMEOUT_SERVER.to_string(),
            marker.to_string_lossy().to_string(),
        ]),
        Duration::from_millis(100),
    )
    .await
    .expect("session should start");

    session
        .call_tool("spawn", json!({}))
        .await
        .expect_err("call should time out");
    tokio::time::sleep(Duration::from_millis(1500)).await;

    assert!(
        !marker.exists(),
        "timed-out stdio session should kill child processes in its process group"
    );
    session.shutdown().await;
}

#[tokio::test]
#[ignore = "requires local apps/desktop test fixture node_modules"]
async fn stdio_ts_sdk_sampling_tools_request_reaches_create_message_handler() {
    let fixture_dir = workspace_root().join("apps/desktop/test-fixtures/mcp-sampling-tool");
    let tsx = fixture_dir.join("node_modules/.bin/tsx");
    let server = fixture_dir.join("multiTurnToolLoopServer.ts");
    assert!(tsx.exists(), "missing fixture-local tsx: {}", tsx.display());
    assert!(
        server.exists(),
        "missing TS SDK sampling fixture: {}",
        server.display()
    );

    let session = StdioSession::start_with_startup_timeout_registry_and_sampling(
        &McpServerStdio::new(
            "multi-turn-tool-loop".to_string(),
            tsx.to_string_lossy().to_string(),
        )
        .args(vec![server.to_string_lossy().to_string()]),
        Duration::from_secs(20),
        Duration::from_secs(20),
        Arc::new(PendingRequestRegistry::new()),
        Arc::new(SamplingSessionRegistry::new()),
        None,
    )
    .await
    .expect("TS SDK sampling fixture should start");

    let result = session
        .call_tool("chain-of-tools", json!({ "iterations": 1 }))
        .await
        .expect("TS SDK fixture should return a tool error after create_message rejects");
    session.shutdown().await;

    assert_eq!(result.is_error, Some(true));
    let message = format!("{result:?}");
    assert!(
        message.contains("No sampling agent configured"),
        "expected daemon create_message handler error after TS SDK accepted sampling.tools, got: {message}"
    );
    assert!(
        !message.contains("Client does not support sampling tools capability"),
        "TS SDK rejected the daemon initialize capabilities before calling create_message: {message}"
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("maenifold-core crate should live under crates/")
        .to_path_buf()
}
