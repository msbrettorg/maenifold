#![allow(dead_code)]

use maenifold_core::{Config, mcp::MaenifoldMcpServer};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const PYTHON: &str = "/usr/bin/python3";
const PROTOCOL_VERSION: &str = "2025-03-26";

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
        write_message({"jsonrpc":"2.0","id":message_id,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{},"resources":{}},"serverInfo":{"name":server_name,"version":"0.0.1"}}})
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        tools = []
        for spec in tool_specs.values():
            tool = {"name": spec["name"], "description": spec.get("description", spec["name"]), "inputSchema": {"type":"object","properties":{}}}
            if "resourceUri" in spec:
                tool["_meta"] = {"ui": {"resourceUri": spec["resourceUri"]}}
            tools.append(tool)
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
    elif method == "resources/read":
        uri = message.get("params", {}).get("uri")
        owner = next((spec for spec in tool_specs.values() if spec.get("resourceUri") == uri), None)
        if owner is None:
            write_message({"jsonrpc":"2.0","id":message_id,"error":{"code":-32002,"message":f"Unknown resource: {uri}"}})
        else:
            write_message({"jsonrpc":"2.0","id":message_id,"result":{"contents":[{"uri":uri,"mimeType":"text/html","text":owner.get("resourceText", "<div>missing</div>")}]}})
    else:
        write_message({"jsonrpc":"2.0","id":message_id,"error":{"code":-32601,"message":f"Unknown method: {method}"}})
"#;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct ConfigGuard;

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct TestServer {
    pub endpoint: String,
    pub client: reqwest::Client,
    shutdown: CancellationToken,
    task: JoinHandle<maenifold_core::Result<()>>,
    _root: TempDir,
    _guard: ConfigGuard,
}

impl TestServer {
    pub async fn start(config: Value) -> Self {
        let root = TempDir::new().expect("temp root");
        std::fs::write(
            root.path().join("maenifold.json"),
            serde_json::to_vec_pretty(&config).expect("config serializes"),
        )
        .expect("write maenifold config");
        Config::override_root(root.path());
        Config::set_database_path(root.path().join("memory.db"));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral listener");
        let addr = listener.local_addr().expect("listener addr");
        let shutdown = CancellationToken::new();
        let task_shutdown = shutdown.clone();
        let task = tokio::spawn(async move {
            MaenifoldMcpServer::serve_http_listener_with_shutdown(listener, task_shutdown).await
        });

        let server = Self {
            endpoint: format!("http://{addr}/mcp"),
            client: reqwest::Client::builder().build().expect("reqwest client"),
            shutdown,
            task,
            _root: root,
            _guard: ConfigGuard,
        };
        server.wait_until_ready().await;
        server
    }

    async fn wait_until_ready(&self) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if tokio::time::Instant::now() > deadline {
                panic!("HTTP MCP listener did not become ready");
            }
            match self.client.get(&self.endpoint).send().await {
                Ok(_) => return,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
            }
        }
    }

    pub async fn stop(self) {
        self.shutdown.cancel();
        let result = tokio::time::timeout(std::time::Duration::from_secs(8), self.task)
            .await
            .expect("server shutdown should complete")
            .expect("server task should not panic");
        result.expect("server should stop cleanly");
    }
}

pub fn ok_tool(name: &str, text: &str) -> Value {
    json!({"name": name, "description": format!("{name} description"), "text": text})
}

pub fn error_tool(name: &str, code: i64, message: &str, data: Value) -> Value {
    json!({
        "name": name,
        "description": format!("{name} description"),
        "error": {"code": code, "message": message, "data": data},
    })
}

pub fn ui_tool(name: &str, text: &str, resource_uri: &str, resource_text: &str) -> Value {
    json!({
        "name": name,
        "description": format!("{name} description"),
        "text": text,
        "resourceUri": resource_uri,
        "resourceText": resource_text,
    })
}

pub fn gateway_config(servers: Vec<Value>) -> Value {
    json!({ "mcpServers": servers, "agents": {} })
}

pub fn stdio_server(name: &str, tool_specs: Vec<Value>) -> Value {
    json!({
        "type": "stdio",
        "name": name,
        "command": PYTHON,
        "args": ["-u", "-c", TEST_SERVER, name, serde_json::to_string(&tool_specs).expect("tool specs serialize")],
        "timeout": 2,
        "gateway": true
    })
}

pub fn broken_server(name: &str) -> Value {
    json!({
        "type": "stdio",
        "name": name,
        "command": "/definitely/not/a/real/mcp/server",
        "args": [],
        "timeout": 1,
        "gateway": true
    })
}

pub fn mcp_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "mcp-protocol-version",
        HeaderValue::from_static(PROTOCOL_VERSION),
    );
    headers
}

pub async fn rpc(
    server: &TestServer,
    method: &str,
    params: Value,
    extra_headers: Option<HeaderMap>,
) -> Value {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut headers = mcp_headers();
    if let Some(extra_headers) = extra_headers {
        headers.extend(extra_headers);
    }
    let body = if params.is_null() {
        json!({"jsonrpc":"2.0","id":id,"method":method})
    } else {
        json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
    };
    let response = server
        .client
        .post(&server.endpoint)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .expect("HTTP RPC request");
    assert!(response.status().is_success(), "{method} HTTP failure");
    response.json().await.expect("JSON-RPC response")
}

pub fn tool_names(response: &Value) -> Vec<String> {
    response["result"]["tools"]
        .as_array()
        .expect("tools/list result")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name").to_string())
        .collect()
}

pub fn result_text(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .expect("tool result text")
}

pub async fn initialize(server: &TestServer) -> Value {
    rpc(
        server,
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "gateway-streamable-http-test", "version": "0.0.1"}
        }),
        None,
    )
    .await
}
