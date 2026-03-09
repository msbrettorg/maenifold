// T-5.6: RTM FR-5.6 (RTM gap — FR-5.6 / MCP protocol conformance is not yet
// tracked in RTM.md; this test suite aligns with NFR-3 regression safety and
// the scaffolded `--mcp` stdio server entrypoint in cli.rs / mcp.rs)
//
//! MCP protocol conformance tests for the `maenifold --mcp` stdio server.
//!
//! These tests spawn the `maenifold` binary with `--mcp` and exercise the MCP
//! JSON-RPC 2.0 protocol over the rmcp stdio transport.
//! They verify:
//!
//!   1. `initialize` returns server info (name + version)
//!   2. `tools/list` returns all 28 tools with input schemas
//!   3. `tools/call` dispatches a known tool and returns a result
//!   4. Unknown tool call returns an MCP-level error result
//!   5. `ServerInstructions` are present and < 500 words
//!   6. Tool names from `tools/list` match `ToolRegistry` canonical names

use maenifold_core::ToolRegistry;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

// T-5.6: RTM FR-5.6 — expected tool count must stay in sync with
// registry.rs DESCRIPTORS (currently 28 tools).
const EXPECTED_TOOL_COUNT: usize = 28;

/// Timeout for receiving a single MCP JSON-RPC response frame.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Manages a `maenifold --mcp` child process and provides helpers for sending
/// JSON-RPC requests and reading stdio-framed MCP responses.
struct McpTestHarness {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    next_id: u64,
}

impl McpTestHarness {
    /// Spawn an isolated `maenifold --mcp` process with a fresh temp root.
    fn spawn(tmp: &TempDir) -> Self {
        let bin = assert_cmd::cargo::cargo_bin!("maenifold");
        let mut child = Command::new(bin)
            .arg("--mcp")
            .env("MAENIFOLD_ROOT", tmp.path())
            .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn maenifold --mcp");

        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        // T-10.18: RTM NFR-3 — rmcp stdio uses newline-delimited JSON-RPC, not
        // `Content-Length` framing, so the harness must read one JSON message
        // per line from stdout.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_jsonrpc_message(&mut reader) {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });

        let (stderr_tx, stderr_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if stderr_tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            child,
            stdin,
            rx,
            stderr_rx,
            next_id: 1,
        }
    }

    /// Send a raw JSON-RPC message using rmcp's newline-delimited stdio
    /// transport.
    fn send_raw(&mut self, msg: &Value) {
        let body = serde_json::to_vec(msg).expect("serialize JSON-RPC message");
        self.stdin
            .write_all(&body)
            .expect("write MCP JSON-RPC body");
        self.stdin
            .write_all(b"\n")
            .expect("write MCP newline delimiter");
        self.stdin.flush().expect("flush child stdin");
    }

    /// Send a JSON-RPC request with auto-incrementing `id` and return the
    /// server's response.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send_raw(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        self.recv()
    }

    /// Read one JSON-RPC response frame (with timeout).
    fn recv(&mut self) -> Value {
        let line = match self.rx.recv_timeout(READ_TIMEOUT) {
            Ok(line) => line,
            Err(err) => panic!(
                "timeout waiting for MCP response: {err}; child_status={:?}; stderr={}",
                self.child.try_wait().ok().flatten(),
                self.drain_stderr()
            ),
        };
        serde_json::from_str(&line).expect("parse MCP response as JSON")
    }

    /// Perform the full MCP initialization handshake and return the JSON-RPC
    /// response envelope (contains `result` with `InitializeResult`).
    fn initialize(&mut self) -> Value {
        let resp = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-conformance-test",
                    "version": "0.0.1"
                }
            }),
        );

        // Per MCP spec the client must send `notifications/initialized` after
        // receiving the initialize result.  No response is expected.
        self.send_raw(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));

        // Brief pause so the server can process the notification before the
        // next request arrives.
        std::thread::sleep(Duration::from_millis(50));

        resp
    }

    fn drain_stderr(&mut self) -> String {
        let mut lines = Vec::new();
        while let Ok(line) = self.stderr_rx.try_recv() {
            if !line.trim().is_empty() {
                lines.push(line);
            }
        }
        if lines.is_empty() {
            "<empty>".to_string()
        } else {
            lines.join(" | ")
        }
    }
}

impl Drop for McpTestHarness {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn isolated_root() -> TempDir {
    TempDir::new().expect("create temp dir")
}

// T-10.18: RTM NFR-3 — rmcp's AsyncRwTransport encodes each JSON-RPC message
// as a single JSON line terminated by `\n`.
fn read_jsonrpc_message(reader: &mut BufReader<impl Read>) -> Option<String> {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return None,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            Err(_) => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Case 1 — `initialize` returns server info with name, version, protocol
/// version, and tool capabilities.
// T-5.6: RTM FR-5.6 — case 1
#[test]
fn mcp_initialize_returns_server_info() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    let resp = h.initialize();

    // JSON-RPC envelope.
    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(
        resp.get("error").is_none(),
        "initialize must not return a JSON-RPC error"
    );

    let result = &resp["result"];

    // serverInfo.name and serverInfo.version.
    let info = &result["serverInfo"];
    assert_eq!(info["name"], "maenifold", "server name mismatch");
    assert!(
        info["version"].is_string(),
        "server version must be a string"
    );

    // protocolVersion.
    assert!(
        result["protocolVersion"].is_string(),
        "protocolVersion must be present as a string"
    );

    // capabilities.tools must be advertised so the client knows it can call
    // tools/list and tools/call.
    assert!(
        result["capabilities"]["tools"].is_object(),
        "tools capability must be advertised"
    );
}

/// Case 2 — `tools/list` returns exactly 28 tools, each with a name,
/// description, and inputSchema.
// T-5.6: RTM FR-5.6 — case 2
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

    // Every tool must carry a name (string), description (string), and
    // inputSchema (object with at least `"type": "object"`).
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

/// Case 3 — `tools/call` dispatches a known, side-effect-free tool (`GetConfig`)
/// and returns a non-error result with content.
// T-5.6: RTM FR-5.6 — case 3
#[test]
fn mcp_tools_call_dispatches_known_tool() {
    let tmp = isolated_root();
    let mut h = McpTestHarness::spawn(&tmp);
    h.initialize();

    // GetConfig is safe and works even against an empty root — it just reads
    // the resolved configuration paths.
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

    // content array must be non-empty.
    let content = result["content"]
        .as_array()
        .expect("result.content must be an array");
    assert!(!content.is_empty(), "result.content must not be empty");

    // isError should be absent or false.
    let is_error = result.get("isError").and_then(Value::as_bool);
    assert_ne!(
        is_error,
        Some(true),
        "GetConfig must not report isError=true"
    );
}

/// Case 4 — calling an unknown tool returns a result with `isError: true`.
/// (The maenifold MCP handler wraps unknown-tool errors in `CallToolResult`
/// rather than returning a JSON-RPC-level error.)
// T-5.6: RTM FR-5.6 — case 4
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

    // The handler returns Ok(CallToolResult { is_error: true, .. }) so the
    // JSON-RPC envelope has `result`, not `error`.
    let result = &resp["result"];
    assert_eq!(
        result["isError"], true,
        "unknown tool must set isError=true in the result"
    );

    // content should mention the error.
    let content = result["content"]
        .as_array()
        .expect("result.content must be an array");
    assert!(
        !content.is_empty(),
        "error result must contain at least one content entry"
    );
}

/// Case 5 — `ServerInstructions` must be present in the initialize result and
/// must be under 500 words.
// T-5.6: RTM FR-5.6 — case 5
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

/// Case 6 — the tool names returned by `tools/list` must exactly match the
/// canonical names from `ToolRegistry::canonical_names()`.
// T-5.6: RTM FR-5.6 — case 6
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
        .map(|t| {
            t["name"]
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
