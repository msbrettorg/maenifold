use super::gateway_startup::build_gateway;
use super::{MaenifoldHandler, TransportKind};
use crate::api::routes::{ApiState, api_routes, endophasia_routes};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_handler::GatewayState;
use axum::Router;
use axum::routing::get;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::sync::Arc;
use tracing::info;

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
    gateway: Option<Arc<GatewayState>>,
) -> MaenifoldHttpService {
    let session_manager = Arc::new(NeverSessionManager::default());

    StreamableHttpService::new(
        move || {
            let mut handler = MaenifoldHandler::new(TransportKind::Http)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            if let Some(gateway) = gateway.as_ref() {
                handler.set_gateway(gateway.clone());
            }
            Ok(handler)
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
        let gateway = build_gateway(&config).await;
        let pool_for_api = gateway.as_ref().map(|gateway| gateway.pool());
        let router = router(
            service(shutdown_token.child_token(), gateway),
            ApiState::new(config.clone(), pool_for_api),
        )
        // T-17.2: RTM FR-23 (RTM-130) — allow browser-based MCP clients to reach the
        // shared tool surface over HTTP.
        // FR-32-SEC: restrict CORS to localhost origins only.
        .layer(super::localhost_cors());
        let bind_addr = tcp_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;
        let endpoint = format!("http://{bind_addr}/mcp");

        info!(endpoint = %endpoint, "maenifold MCP HTTP server bound");

        let graceful_shutdown_token = shutdown_token.clone();
        let shutdown_endpoint = endpoint.clone();
        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = graceful_shutdown_token.cancelled() => {}
                }
                info!(endpoint = %shutdown_endpoint, "maenifold MCP HTTP server shutting down");
                // Bounded drain: give open connections 3 seconds to close,
                // then force-stop so block_on returns and shutdown_timeout is reachable.
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            })
            .await
            .map_err(|e| MaenifoldError::Message(format!("HTTP server error: {e}")))?;

        info!(endpoint = %endpoint, "maenifold MCP HTTP server shutdown complete");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{router, service};
    use crate::api::routes::ApiState;
    use crate::mcp::localhost_cors;
    use crate::test_util::ConfigResetGuard;
    use crate::{Config, GraphDatabase};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_state() -> (TempDir, ApiState, ConfigResetGuard) {
        let temp = TempDir::new().expect("temp dir");
        Config::override_root(temp.path());
        Config::set_database_path(temp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        GraphDatabase::initialize_database(&config).expect("initialize database");
        (temp, ApiState::new(config, None), ConfigResetGuard)
    }

    fn test_router(state: ApiState) -> axum::Router {
        router(
            service(tokio_util::sync::CancellationToken::new(), None),
            state,
        )
            .layer(localhost_cors())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn router_serves_endophasia_frontend() {
        let (_temp, state, _guard) = test_state();
        let response = test_router(state)
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
        let response = test_router(state)
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

    #[tokio::test]
    #[serial_test::serial]
    async fn router_allows_packaged_electron_null_origin() {
        let (_temp, state, _guard) = test_state();
        let response = test_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .header(header::ORIGIN, "null")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "null"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn router_allows_packaged_electron_file_origin() {
        let (_temp, state, _guard) = test_state();
        let response = test_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .header(header::ORIGIN, "file://")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "file://"
        );
    }
}
