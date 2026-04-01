// T-17.5: RTM FR-23 (RTM-127, RTM-129, RTM-130)
use maenifold_core::mcp::MaenifoldMcpServer;
use maenifold_core::{Config, ToolRegistry};
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

    async fn initialize_session(&self) -> (Value, String) {
        let response = self.send_mcp_request(initialize_request(), None).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "initialize should succeed"
        );

        let session_id = extract_session_id(&response);
        let body = parse_mcp_response(response).await;
        self.send_initialized_notification(&session_id).await;
        (body, session_id)
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

    async fn open_sse_session(&self, session_id: &str) -> Response {
        self.client
            .get(self.endpoint())
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .expect("HTTP GET /mcp should complete")
    }

    async fn delete_session(&self, session_id: &str) -> Response {
        self.client
            .delete(self.endpoint())
            .header("Accept", ACCEPT_HEADER)
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .expect("HTTP DELETE /mcp should complete")
    }

    async fn send_initialized_notification(&self, session_id: &str) {
        let response = self
            .send_mcp_request(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }),
                Some(session_id),
            )
            .await;

        assert_eq!(
            response.status(),
            StatusCode::ACCEPTED,
            "notifications/initialized should complete the MCP handshake"
        );
    }
}

impl Drop for HttpTestServer {
    fn drop(&mut self) {
        self.handle.abort();
        Config::reset_overrides();
    }
}

// T-17.5: RTM FR-23 (RTM-127) — POST /mcp initializes the MCP session, GET /mcp accepts the
// returned session header for SSE, and DELETE /mcp terminates the session.
#[tokio::test]
async fn http_transport_initialize_returns_server_info_and_supports_session_endpoints() {
    let server = HttpTestServer::start().await;

    let (initialize_body, session_id) = server.initialize_session().await;
    assert_eq!(initialize_body["jsonrpc"], "2.0");
    assert_eq!(initialize_body["id"], INIT_REQUEST_ID);
    assert_eq!(initialize_body["result"]["serverInfo"]["name"], "maenifold");
    assert!(
        initialize_body["result"]["serverInfo"]["version"]
            .as_str()
            .is_some_and(|version| !version.is_empty()),
        "initialize should return a non-empty server version: {initialize_body}"
    );
    assert!(
        initialize_body["result"]["capabilities"]["tools"].is_object(),
        "initialize should advertise tools capability: {initialize_body}"
    );

    let get_response = server.open_sse_session(&session_id).await;
    assert_eq!(get_response.status(), StatusCode::OK);
    assert!(
        content_type(&get_response).contains("text/event-stream"),
        "GET /mcp should open an SSE stream"
    );
    drop(get_response);

    let delete_response = server.delete_session(&session_id).await;
    assert_eq!(delete_response.status(), StatusCode::ACCEPTED);
}

// T-17.5: RTM FR-23 (RTM-129) — follow-up MCP requests honor `Mcp-Session-Id`, and unknown
// session IDs are rejected with 404 per the Streamable HTTP session contract.
#[tokio::test]
async fn http_transport_validates_session_ids_on_follow_up_requests() {
    let server = HttpTestServer::start().await;
    let (_initialize_body, session_id) = server.initialize_session().await;

    let tools_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
            Some(&session_id),
        )
        .await;
    assert_eq!(tools_response.status(), StatusCode::OK);

    let tools_body = parse_mcp_response(tools_response).await;
    let tool_names = tools_body["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tool array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    let expected_tool_count = ToolRegistry::new().canonical_names().len();
    // T-17.5: RTM FR-23 (RTM-130) — assert the full canonical registry is exposed over HTTP.
    assert_eq!(
        tool_names.len(),
        expected_tool_count,
        "RTM-130: all registered tools must be exposed over HTTP"
    );
    assert!(
        tool_names.contains(&"GetConfig"),
        "tools/list should include GetConfig: {tools_body}"
    );

    let stale_response = server
        .send_mcp_request(
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/list",
                "params": {}
            }),
            Some("stale-session-id"),
        )
        .await;
    assert_eq!(stale_response.status(), StatusCode::NOT_FOUND);

    let stale_body = stale_response
        .text()
        .await
        .expect("404 body should be readable");
    assert!(
        stale_body
            .to_ascii_lowercase()
            .contains("session not found"),
        "stale session response should explain the failure: {stale_body}"
    );
}

// T-17.5: RTM FR-23 (RTM-130) — tools/call over Streamable HTTP reaches the live tool registry
// and returns a successful tool payload through the MCP response envelope.
#[tokio::test]
async fn http_transport_invokes_tools_over_http() {
    let server = HttpTestServer::start().await;
    let (_initialize_body, session_id) = server.initialize_session().await;

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
            Some(&session_id),
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

fn extract_session_id(response: &Response) -> String {
    response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .expect("initialize response should include Mcp-Session-Id")
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
