use super::{StdioSession, read_response};
use crate::error::{GATEWAY_TIMEOUT_ERROR, MaenifoldError};
use agent_client_protocol_schema::McpServerStdio;
use serde_json::json;
use tokio::io::{AsyncWriteExt, BufReader};
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

#[tokio::test]
async fn stdio_session_timeout_poisoning_blocks_followup_calls() {
    let session = StdioSession::start(
        &McpServerStdio::new("slow-server".to_string(), PYTHON.to_string())
            .args(vec!["-u".to_string(), "-c".to_string(), TIMEOUT_SERVER.to_string()]),
        Duration::from_millis(100),
    )
    .await
    .expect("session should start");

    match session.call_tool("slow", json!({})).await.expect_err("call should time out") {
        MaenifoldError::JsonRpc { code, .. } => assert_eq!(code, GATEWAY_TIMEOUT_ERROR),
        other => panic!("unexpected error: {other}"),
    }
    assert!(session.is_poisoned());
    match session.call_tool("slow", json!({})).await.expect_err("poisoned session should reject follow-up calls") {
        MaenifoldError::SessionUnusable => {}
        other => panic!("unexpected error: {other}"),
    }
    session.shutdown().await;
}

#[tokio::test]
async fn stdio_session_reports_json_rpc_error_without_code_field() {
    let (mut writer, reader) = tokio::io::duplex(256);
    writer
        .write_all(br#"{"jsonrpc":"2.0","id":1,"error":{"message":"fail"}}"#)
        .await
        .expect("write payload");
    writer.write_all(b"\n").await.expect("write newline");
    drop(writer);

    let mut reader = BufReader::new(reader);
    let error = read_response(&mut reader, 1)
        .await
        .expect_err("missing code should fail");
    assert_eq!(
        error.to_string(),
        r#"JSON-RPC error missing integer code: {"message":"fail"}"#
    );
}
