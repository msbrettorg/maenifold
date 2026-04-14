use super::{MaenifoldHandler, TransportKind};
use crate::api::routes::{ApiState, api_routes, endophasia_routes};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use axum::Router;
use axum::routing::get;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::sync::Arc;

pub(super) type MaenifoldHttpService = StreamableHttpService<MaenifoldHandler, NeverSessionManager>;

pub(super) fn router(service: MaenifoldHttpService, state: ApiState) -> Router {
    let relay_state = state.clone();
    Router::new()
        .nest_service("/mcp", service)
        .merge(api_routes(state.clone()))
        .merge(endophasia_routes(state))
        .route(
            "/api/acp/session",
            get(crate::api::acp_relay::ws_handler).with_state(relay_state),
        )
}

// Stateless MCP service — no session affinity, no keepalive timeout, no
// stale-session errors. Every tool call is independent. Clients may send
// an `Mcp-Session-Id` header but the server ignores it and never rejects
// requests for session reasons. This eliminates the entire class of
// "session_expired" failures that occur when long-running AI workloads
// idle beyond the keepalive window.
pub(super) fn service(
    cancellation_token: tokio_util::sync::CancellationToken,
) -> MaenifoldHttpService {
    let session_manager = Arc::new(NeverSessionManager::default());

    StreamableHttpService::new(
        || {
            MaenifoldHandler::new(TransportKind::Http)
                .map_err(|e| std::io::Error::other(e.to_string()))
        },
        session_manager,
        StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true)
            .with_cancellation_token(cancellation_token),
    )
}

impl super::MaenifoldMcpServer {
    // T-17.2: RTM FR-23 (RTM-127, RTM-129, RTM-130) — expose Streamable HTTP MCP
    // endpoints with rmcp session management and browser-friendly CORS.
    pub async fn serve_http(port: u16) -> Result<()> {
        Self::serve_http_with_shutdown(port, tokio_util::sync::CancellationToken::new()).await
    }

    pub async fn serve_http_with_shutdown(
        port: u16,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {bind_addr}: {e}"))
            })?;

        Self::serve_http_listener_with_shutdown(tcp_listener, shutdown_token).await
    }

    pub async fn serve_http_listener(tcp_listener: tokio::net::TcpListener) -> Result<()> {
        Self::serve_http_listener_with_shutdown(
            tcp_listener,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
    }

    async fn serve_http_listener_with_shutdown(
        tcp_listener: tokio::net::TcpListener,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let config = Config::current();
        super::initialize_runtime(&config)?;
        let router = router(
            service(shutdown_token.child_token()),
            ApiState::new(config.clone()),
        )
        // T-17.2: RTM FR-23 (RTM-130) — allow browser-based MCP clients to reach the
        // shared tool surface over HTTP.
        // FR-32-SEC: restrict CORS to localhost origins only.
        .layer(super::localhost_cors());
        let bind_addr = tcp_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;

        eprintln!("maenifold MCP HTTP server listening on http://{bind_addr}/mcp");

        let graceful_shutdown_token = shutdown_token.clone();
        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = graceful_shutdown_token.cancelled() => {}
                }
                // Bounded drain: give open connections 3 seconds to close,
                // then force-stop so block_on returns and shutdown_timeout is reachable.
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            })
            .await
            .map_err(|e| MaenifoldError::Message(format!("HTTP server error: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{router, service};
    use crate::api::routes::ApiState;
    use crate::{Config, GraphDatabase};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tempfile::TempDir;
    use tower::ServiceExt;

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn test_state() -> (TempDir, ApiState, ConfigResetGuard) {
        let temp = TempDir::new().expect("temp dir");
        Config::override_root(temp.path());
        Config::set_database_path(temp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        GraphDatabase::initialize_database(&config).expect("initialize database");
        (temp, ApiState::new(config), ConfigResetGuard)
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn router_serves_endophasia_frontend() {
        let (_temp, state, _guard) = test_state();
        let response = router(service(tokio_util::sync::CancellationToken::new()), state)
            .oneshot(
                Request::builder()
                    .uri("/endophasia")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(body.contains("Endophasia"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn router_serves_endophasia_communities_json() {
        let (_temp, state, _guard) = test_state();
        let response = router(service(tokio_util::sync::CancellationToken::new()), state)
            .oneshot(
                Request::builder()
                    .uri("/api/endophasia/communities")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["communities"], serde_json::json!([]));
        assert_eq!(json["edges"], serde_json::json!([]));
    }
}
