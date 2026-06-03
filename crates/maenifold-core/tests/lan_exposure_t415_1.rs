use axum::{
    body::Body,
    http::{
        Request, StatusCode,
        header::{ACCEPT, CONNECTION, CONTENT_TYPE, HOST, ORIGIN, UPGRADE},
    },
};
use maenifold_core::{
    Config, GraphDatabase,
    mcp::{MaenifoldMcpServer, http::{local_api_router, mcp_router}},
};
use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;
use tokio::{net::TcpListener, sync::oneshot};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

struct ConfigGuard;

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn initialize_config(config_body: serde_json::Value) -> (TempDir, Config, ConfigGuard) {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(
        temp.path().join("maenifold.json"),
        serde_json::to_vec(&config_body).expect("config json"),
    )
    .expect("write config");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    GraphDatabase::initialize_database(&config).expect("initialize database");
    (temp, config, ConfigGuard)
}

fn minimal_config_json() -> serde_json::Value {
    json!({
        "agents": {},
        "settings": {}
    })
}

fn initialize_request_body() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "lan-exposure-test",
                "version": "0.1.0"
            }
        }
    })
}

fn initialize_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(ACCEPT, "application/json, text/event-stream")
        .header(CONTENT_TYPE, "application/json")
        .header("mcp-protocol-version", "2025-03-26")
        .body(Body::from(initialize_request_body().to_string()))
        .expect("request")
}

async fn spawn_http_listener(
    config_body: serde_json::Value,
) -> (TempDir, ConfigGuard, CancellationToken, tokio::task::JoinHandle<maenifold_core::Result<()>>, std::net::SocketAddr) {
    let (temp, _config, guard) = initialize_config(config_body);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral listener");
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = CancellationToken::new();
    let task_shutdown = shutdown.clone();
    let (ready_tx, ready_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = ready_tx.send(());
        MaenifoldMcpServer::serve_http_listener_with_shutdown(listener, task_shutdown).await
    });
    ready_rx.await.expect("server ready signal");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (temp, guard, shutdown, task, addr)
}

async fn post_mcp(
    addr: std::net::SocketAddr,
    host: Option<String>,
    origin: Option<&str>,
) -> reqwest::Response {
    let client = reqwest::Client::builder()
        .build()
        .expect("reqwest client");
    let mut request = client
        .post(format!("http://{addr}/mcp"))
        .header(ACCEPT, "application/json, text/event-stream")
        .header(CONTENT_TYPE, "application/json")
        .header("mcp-protocol-version", "2025-03-26")
        .body(initialize_request_body().to_string());
    if let Some(host) = host {
        request = request.header(HOST, host);
    }
    if let Some(origin) = origin {
        request = request.header(ORIGIN, origin);
    }
    request.send().await.expect("POST /mcp response")
}

#[tokio::test]
#[serial]
async fn mcp_router_returns_404_for_api_health() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = mcp_router(&config)
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn mcp_router_returns_404_for_api_config() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = mcp_router(&config)
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn mcp_router_returns_404_for_api_acp_session() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = mcp_router(&config)
        .oneshot(
            Request::builder()
                .uri("/api/acp/session")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn mcp_router_serves_mcp_route() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = mcp_router(&config)
        .oneshot(initialize_request("/mcp"))
        .await
        .expect("response");
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn local_api_router_returns_404_for_mcp() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn local_api_router_serves_api_health() {
    let (_temp, config, _guard) = initialize_config(minimal_config_json());
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn mcp_listener_blocks_bad_host_with_default_allowed_hosts() {
    let (_temp, _guard, shutdown, task, addr) = spawn_http_listener(minimal_config_json()).await;
    let response = post_mcp(addr, Some("evil.example".to_string()), None).await;
    shutdown.cancel();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(8), task)
        .await
        .expect("server shutdown")
        .expect("server task");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn mcp_listener_allows_loopback_host_with_default_allowed_hosts() {
    let (_temp, _guard, shutdown, task, addr) = spawn_http_listener(minimal_config_json()).await;
    let response = post_mcp(addr, Some(format!("127.0.0.1:{}", addr.port())), None).await;
    shutdown.cancel();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(8), task)
        .await
        .expect("server shutdown")
        .expect("server task");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn mcp_listener_allows_any_origin_with_empty_allowed_origins() {
    let (_temp, _guard, shutdown, task, addr) = spawn_http_listener(minimal_config_json()).await;
    let response = post_mcp(addr, None, Some("https://evil.example")).await;
    shutdown.cancel();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(8), task)
        .await
        .expect("server shutdown")
        .expect("server task");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn mcp_listener_blocks_bad_origin_when_allowed_origins_populated() {
    let (_temp, _guard, shutdown, task, addr) = spawn_http_listener(json!({
        "agents": {},
        "settings": {},
        "mcpAllowedOrigins": ["https://allowed.example"]
    }))
    .await;
    let response = post_mcp(addr, None, Some("https://evil.example")).await;
    shutdown.cancel();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(8), task)
        .await
        .expect("server shutdown")
        .expect("server task");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn mcp_listener_allows_allowlisted_origin() {
    let (_temp, _guard, shutdown, task, addr) = spawn_http_listener(json!({
        "agents": {},
        "settings": {},
        "mcpAllowedOrigins": ["https://allowed.example"]
    }))
    .await;
    let response = post_mcp(addr, None, Some("https://allowed.example")).await;
    shutdown.cancel();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(8), task)
        .await
        .expect("server shutdown")
        .expect("server task");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn acp_ws_rejects_origin_not_in_allowlist() {
    let (_temp, mut config, _guard) = initialize_config(minimal_config_json());
    config.mcp_allowed_origins = vec!["https://allowed.example".to_string()];
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/acp/session?agent=Copilot")
                .header(CONNECTION, "Upgrade")
                .header(UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header(ORIGIN, "https://evil.example")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn acp_ws_allows_origin_in_allowlist() {
    let (_temp, mut config, _guard) = initialize_config(minimal_config_json());
    config.mcp_allowed_origins = vec!["https://allowed.example".to_string()];
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/acp/session?agent=Copilot")
                .header(CONNECTION, "Upgrade")
                .header(UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header(ORIGIN, "https://allowed.example")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn acp_ws_allows_when_origin_absent() {
    let (_temp, mut config, _guard) = initialize_config(minimal_config_json());
    config.mcp_allowed_origins = vec!["https://allowed.example".to_string()];
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/acp/session?agent=Copilot")
                .header(CONNECTION, "Upgrade")
                .header(UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

// Red-team T-415.1 (critical): Origin: null from a sandboxed iframe MUST be
// rejected when an allowlist is configured. Hardcoded first-party
// acceptance of opaque origins is a DNS-rebinding bypass: any attacker
// page can synthesize `Origin: null` via a sandboxed iframe and reach the
// ACP WebSocket on loopback, which spawns CLI subprocesses under the
// operator's UID. If a user really needs to allow these, they must list
// them explicitly in `mcpAllowedOrigins`.
#[tokio::test]
#[serial]
async fn acp_ws_rejects_opaque_null_origin() {
    let (_temp, mut config, _guard) = initialize_config(minimal_config_json());
    config.mcp_allowed_origins = vec!["https://allowed.example".to_string()];
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/acp/session?agent=Copilot")
                .header(CONNECTION, "Upgrade")
                .header(UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header(ORIGIN, "null")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn acp_ws_rejects_opaque_file_origin() {
    let (_temp, mut config, _guard) = initialize_config(minimal_config_json());
    config.mcp_allowed_origins = vec!["https://allowed.example".to_string()];
    let response = local_api_router(&config)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/acp/session?agent=Copilot")
                .header(CONNECTION, "Upgrade")
                .header(UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header(ORIGIN, "file://")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
