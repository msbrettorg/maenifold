// T-10.18: RTM NFR-3
#![allow(dead_code)]
//! Shared helpers for MCP stdio integration tests split from `mcp_tests.rs`.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

// T-10.18: RTM NFR-3 — expected tool count must stay in sync with
// registry.rs DESCRIPTORS (currently 28 tools).
pub const EXPECTED_TOOL_COUNT: usize = 28;

/// Timeout for receiving a single MCP JSON-RPC response frame.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Manages a `maenifold --mcp` child process and provides helpers for sending
/// JSON-RPC requests and reading stdio-framed MCP responses.
pub struct McpTestHarness {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    next_id: u64,
}

impl McpTestHarness {
    // T-10.18: RTM NFR-3 — spawn an isolated `maenifold --mcp` process with a
    // fresh temp root so protocol tests do not depend on ambient user state.
    pub fn spawn(tmp: &TempDir) -> Self {
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
    pub fn send_raw(&mut self, msg: &Value) {
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
    pub fn request(&mut self, method: &str, params: Value) -> Value {
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
    pub fn recv(&mut self) -> Value {
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
    pub fn initialize(&mut self) -> Value {
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

        self.send_raw(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
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

pub fn isolated_root() -> TempDir {
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
