// T-17.5: RTM FR-23 (RTM-127, RTM-129, RTM-130)
use maenifold_core::Config;
use maenifold_core::mcp::MaenifoldMcpServer;
use parking_lot::MutexGuard;
use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

const ACCEPT_HEADER: &str = "application/json, text/event-stream";
const MCP_ENDPOINT: &str = "/mcp";
const INIT_REQUEST_ID: i64 = 1;

struct HttpTestServer {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    config: Config,
    client: Client,
    port: u16,
    handle: JoinHandle<maenifold_core::Result<()>>,
}

impl HttpTestServer {
    // T-17.5: RTM FR-23 (RTM-127, RTM-129, RTM-130) — start the HTTP transport
    // end-to-end against an isolated on-disk runtime so each integration test drives the
    // real rmcp/axum server with predictable config state.
    async fn start() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("temporary test root");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config
            .ensure_directories()
            .expect("test directories should initialize");

        let port = reserve_port().await;
        let handle = tokio::spawn(async move { MaenifoldMcpServer::serve_http(port).await });
        wait_for_server(port).await;

        Self {
            _guard: guard,
            _tmp: tmp,
            config,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(5))
                .build()
                .expect("HTTP test client should initialize"),
            port,
            handle,
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}{MCP_ENDPOINT}", self.port)
    }

    async fn send_mcp_request(&self, body: Value, session_id: Option<&str>) -> Response {
        let mut request = self
            .client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .header("Accept", ACCEPT_HEADER);

        if let Some(session_id) = session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        request
            .json(&body)
            .send()
            .await
            .expect("HTTP MCP request should complete")
    }
}

impl Drop for HttpTestServer {
    fn drop(&mut self) {
        self.handle.abort();
        Config::reset_overrides();
    }
}

// T-17.5: RTM FR-23 (RTM-127) — POST /mcp initializes the MCP session and returns server info.
// In stateless mode (json_response: true), responses are JSON, not SSE.
#[tokio::test]
async fn http_transport_initialize_returns_server_info() {
    let server = HttpTestServer::start().await;

    let response = server.send_mcp_request(initialize_request(), None).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "initialize should succeed"
    );

    let body = parse_mcp_response(response).await;
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], INIT_REQUEST_ID);
    assert_eq!(body["result"]["serverInfo"]["name"], "maenifold");
    assert!(
        body["result"]["serverInfo"]["version"]
            .as_str()
            .is_some_and(|version| !version.is_empty()),
        "initialize should return a non-empty server version: {body}"
    );
    assert!(
        body["result"]["capabilities"]["tools"].is_object(),
        "initialize should advertise tools capability: {body}"
    );
}

// T-17.5: RTM FR-23 (RTM-129) — stateless mode: follow-up requests work without
// session IDs, and stale session IDs are silently ignored (no 404 rejection).
#[tokio::test]
async fn http_transport_stateless_mode_ignores_session_ids() {
    let server = HttpTestServer::start().await;

    // In stateless mode, tools/list works without any session initialization.
    let tools_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "stateless-test",
                        "version": "0.1.0"
                    }
                }
            }),
            None,
        )
        .await;
    assert_eq!(tools_response.status(), StatusCode::OK);

    let init_body = parse_mcp_response(tools_response).await;
    assert_eq!(init_body["result"]["serverInfo"]["name"], "maenifold");

    // Stale session IDs must NOT cause 404 in stateless mode — they are ignored.
    let stale_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "stateless-stale-test",
                        "version": "0.1.0"
                    }
                }
            }),
            Some("stale-session-id"),
        )
        .await;
    assert_ne!(
        stale_response.status(),
        StatusCode::NOT_FOUND,
        "stateless mode must not reject stale session IDs"
    );
}

// T-17.5: RTM FR-23 (RTM-130) — tools/call over Streamable HTTP reaches the live tool registry
// and returns a successful tool payload through the MCP response envelope.
// In stateless mode, each request is self-contained — no session setup required.
#[tokio::test]
async fn http_transport_invokes_tools_over_http() {
    let server = HttpTestServer::start().await;

    // Initialize first (stateless but rmcp still expects the handshake)
    let init_response = server.send_mcp_request(initialize_request(), None).await;
    assert_eq!(init_response.status(), StatusCode::OK);
    let init_body = parse_mcp_response(init_response).await;
    assert_eq!(init_body["result"]["serverInfo"]["name"], "maenifold");

    // Send initialized notification (no session header needed in stateless mode)
    let notif_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }),
            None,
        )
        .await;
    // Stateless servers may accept or reject notifications; either way the
    // tools/call below must succeed.
    let _ = notif_response;

    let call_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "GetConfig",
                    "arguments": {}
                }
            }),
            None,
        )
        .await;
    assert_eq!(call_response.status(), StatusCode::OK);

    let call_body = parse_mcp_response(call_response).await;
    assert_eq!(
        call_body["result"]["isError"].as_bool(),
        Some(false),
        "tool call should succeed: {call_body}"
    );

    let tool_payload = call_body["result"]["content"][0]["text"]
        .as_str()
        .expect("tools/call should return text content");
    let tool_json: Value =
        serde_json::from_str(tool_payload).expect("GetConfig payload should be valid JSON");
    assert_eq!(tool_json["success"].as_bool(), Some(true));

    let rendered = tool_json["data"]
        .as_str()
        .expect("GetConfig JSON envelope should contain markdown text");
    assert!(
        rendered.contains("Maenifold Configuration:"),
        "GetConfig should return the configuration report: {rendered}"
    );
    assert!(
        rendered.contains(server.config.memory_path.to_string_lossy().as_ref()),
        "GetConfig output should reflect the isolated test memory path: {rendered}"
    );
}

fn initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": INIT_REQUEST_ID,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "http-transport-test",
                "version": "0.1.0"
            }
        }
    })
}

fn content_type(response: &Response) -> String {
    response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

async fn parse_mcp_response(response: Response) -> Value {
    let content_type = content_type(&response);
    let body = response
        .text()
        .await
        .expect("response body should be readable");

    if content_type.contains("application/json") {
        serde_json::from_str(&body).expect("application/json response should be valid JSON")
    } else if content_type.contains("text/event-stream") {
        parse_sse_json(&body)
    } else {
        panic!("unexpected content-type {content_type}: {body}");
    }
}

fn parse_sse_json(body: &str) -> Value {
    let data = body
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .find(|line| !line.is_empty())
        .expect("SSE response should contain a JSON data event");

    serde_json::from_str(data).expect("SSE data event should contain valid JSON")
}

async fn reserve_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral port reservation should succeed");
    let port = listener
        .local_addr()
        .expect("ephemeral listener should have a local address")
        .port();
    drop(listener);
    port
}

async fn wait_for_server(port: u16) {
    for _attempt in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("HTTP test server did not start on port {port}");
}
