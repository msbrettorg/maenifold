use super::StdioSession;
use crate::error::{GATEWAY_TIMEOUT_ERROR, MaenifoldError};
use crate::mcp::stdio_io::read_response;
use agent_client_protocol_schema::McpServerStdio;
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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
async fn stdio_session_reports_json_rpc_error_without_code_field() {
    let (mut client_writer, _server_stdin) = tokio::io::duplex(256);
    let (mut server_stdout_writer, server_stdout_reader) = tokio::io::duplex(256);
    server_stdout_writer
        .write_all(br#"{"jsonrpc":"2.0","id":1,"error":{"message":"fail"}}"#)
        .await
        .expect("write payload");
    server_stdout_writer
        .write_all(b"\n")
        .await
        .expect("write newline");
    drop(server_stdout_writer);

    let mut reader = BufReader::new(server_stdout_reader);
    let error = read_response(&mut client_writer, &mut reader, 1, "test-server")
        .await
        .expect_err("missing code should fail");
    assert_eq!(
        error.to_string(),
        r#"JSON-RPC error missing integer code: {"message":"fail"}"#
    );
}

#[tokio::test]
async fn stdio_session_replies_method_not_found_to_server_requests_and_skips_notifications() {
    let (mut client_writer, server_stdin) = tokio::io::duplex(1024);
    let (mut server_stdout_writer, server_stdout_reader) = tokio::io::duplex(1024);

    server_stdout_writer
        .write_all(br#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"message":"skip me"}}"#)
        .await
        .expect("write notification");
    server_stdout_writer
        .write_all(b"\n")
        .await
        .expect("write notification newline");
    server_stdout_writer
        .write_all(br#"{"jsonrpc":"2.0","id":99,"method":"roots/list","params":{}}"#)
        .await
        .expect("write server request");
    server_stdout_writer
        .write_all(b"\n")
        .await
        .expect("write server request newline");
    server_stdout_writer
        .write_all(br#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#)
        .await
        .expect("write matching response");
    server_stdout_writer
        .write_all(b"\n")
        .await
        .expect("write matching response newline");

    let mut reader = BufReader::new(server_stdout_reader);
    let result = read_response(&mut client_writer, &mut reader, 1, "test-server")
        .await
        .expect("matching response should be returned");
    assert_eq!(result, json!({"ok": true}));

    let mut server_stdin_reader = BufReader::new(server_stdin);
    let mut response_line = String::new();
    server_stdin_reader
        .read_line(&mut response_line)
        .await
        .expect("read method-not-found response");
    let response: serde_json::Value =
        serde_json::from_str(response_line.trim()).expect("parse response");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 99);
    assert_eq!(response["error"]["code"], -32601);
    assert_eq!(
        response["error"]["message"],
        "Method not found: client does not implement 'roots/list'"
    );

    let mut extra_line = String::new();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(25),
            server_stdin_reader.read_line(&mut extra_line)
        )
        .await
        .is_err(),
        "notification should not produce a client response"
    );
}
