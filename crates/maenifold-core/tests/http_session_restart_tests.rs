use maenifold_core::Config;
use maenifold_core::mcp::MaenifoldMcpServer;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use reqwest::{Client, Response, StatusCode};
use serde_json::json;
use tempfile::TempDir;
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::{Duration, sleep},
};

static GLOBAL_CONFIG_TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const ACCEPT_HEADER: &str = "application/json, text/event-stream";
const MCP_ENDPOINT: &str = "/mcp";

struct HttpTestServer {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    client: Client,
    port: u16,
    handle: Option<JoinHandle<maenifold_core::Result<()>>>,
}

impl HttpTestServer {
    async fn start() -> Self {
        let guard = GLOBAL_CONFIG_TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("temporary test root");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        Config::current()
            .ensure_directories()
            .expect("test directories should initialize");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ephemeral port should bind");
        let port = listener
            .local_addr()
            .expect("ephemeral listener should expose local addr")
            .port();
        let handle = Some(spawn_server(listener));
        wait_for_server(port).await;

        Self {
            _guard: guard,
            _tmp: tmp,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(5))
                .build()
                .expect("HTTP test client should initialize"),
            port,
            handle,
        }
    }

    async fn restart(&mut self) {
        self.stop().await;
        let listener = TcpListener::bind(("127.0.0.1", self.port))
            .await
            .expect("restart listener should bind");
        self.handle = Some(spawn_server(listener));
        wait_for_server(self.port).await;
    }

    async fn initialize_session(&self) -> String {
        let response = self
            .client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .header("Accept", ACCEPT_HEADER)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "http-session-restart-test",
                        "version": "0.1.0"
                    }
                }
            }))
            .send()
            .await
            .expect("initialize request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .expect("initialize response should include Mcp-Session-Id")
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

    async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = handle.await;
            wait_for_shutdown(self.port).await;
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}{MCP_ENDPOINT}", self.port)
    }
}

impl Drop for HttpTestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Config::reset_overrides();
    }
}

#[tokio::test]
async fn stale_streamable_http_session_after_restart_returns_reinitialize_hint() {
    let mut server = HttpTestServer::start().await;
    let session_id = server.initialize_session().await;

    server.restart().await;

    let response = server.open_sse_session(&session_id).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response
        .text()
        .await
        .expect("stale-session response body should be readable");
    assert!(
        body.contains("Re-initialize"),
        "stale-session response should tell clients to re-initialize: {body}"
    );
    assert!(
        body.contains("Mcp-Session-Id"),
        "stale-session response should explain that a new session id is required: {body}"
    );
}

fn spawn_server(listener: TcpListener) -> JoinHandle<maenifold_core::Result<()>> {
    tokio::spawn(async move { MaenifoldMcpServer::serve_http_listener(listener).await })
}

async fn wait_for_server(port: u16) {
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("server did not start on port {port}");
}

async fn wait_for_shutdown(port: u16) {
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_err() {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("server did not stop on port {port}");
}
