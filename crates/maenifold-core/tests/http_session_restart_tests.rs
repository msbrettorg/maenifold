use maenifold_core::Config;
use maenifold_core::mcp::MaenifoldMcpServer;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::{Duration, sleep},
};

static GLOBAL_CONFIG_TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

    async fn send_initialize(&self) -> Value {
        let response = self
            .client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
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
            .json()
            .await
            .expect("initialize response should parse as JSON")
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

// Stateless mode: after a server restart, clients can immediately send new
// requests without any session-expired error. This is the whole point of the
// NeverSessionManager + stateful_mode: false configuration.
#[tokio::test]
async fn stateless_http_survives_server_restart() {
    let mut server = HttpTestServer::start().await;

    let body_before = server.send_initialize().await;
    assert_eq!(body_before["result"]["serverInfo"]["name"], "maenifold");

    server.restart().await;

    // After restart, a fresh initialize must succeed — no stale session errors.
    let body_after = server.send_initialize().await;
    assert_eq!(body_after["result"]["serverInfo"]["name"], "maenifold");
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
