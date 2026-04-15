use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tracing::info;

use super::{GateMcpHandler, TransportKind};
use crate::error::GateError;

pub(crate) type GateHttpService = StreamableHttpService<GateMcpHandler, LocalSessionManager>;

pub(crate) fn service(
    cancellation_token: tokio_util::sync::CancellationToken,
    known_roots: super::KnownRoots,
) -> GateHttpService {
    let session_manager = Arc::new(LocalSessionManager::default());
    let mut config = StreamableHttpServerConfig::default();
    config.stateful_mode = true;
    config.cancellation_token = cancellation_token;
    StreamableHttpService::new(
        move || {
            GateMcpHandler::new(TransportKind::Http, known_roots.clone())
                .map_err(|e| std::io::Error::other(e.to_string()))
        },
        session_manager,
        config,
    )
}

impl super::GateMcpServer {
    pub async fn serve_http(port: u16, known_roots: super::KnownRoots) -> Result<(), GateError> {
        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| GateError::Cli(format!("HTTP bind failed on {bind_addr}: {e}")))?;
        Self::serve_http_listener(tcp_listener, known_roots).await
    }

    pub async fn serve_http_listener(
        tcp_listener: tokio::net::TcpListener,
        known_roots: super::KnownRoots,
    ) -> Result<(), GateError> {
        let ct = tokio_util::sync::CancellationToken::new();
        let bind_addr = tcp_listener
            .local_addr()
            .map_err(|e| GateError::Cli(format!("HTTP listener address lookup failed: {e}")))?;
        let endpoint = format!("http://{bind_addr}/mcp");
        let router = axum::Router::new()
            .nest_service("/mcp", service(ct.child_token(), known_roots))
            .layer(super::localhost_cors());

        info!(endpoint = %endpoint, "gate MCP HTTP server bound");

        let shutdown_endpoint = endpoint.clone();
        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                info!(endpoint = %shutdown_endpoint, "gate MCP HTTP server shutting down");
                ct.cancel();
            })
            .await
            .map_err(|e| GateError::Cli(format!("HTTP server error: {e}")))?;

        info!(endpoint = %endpoint, "gate MCP HTTP server shutdown complete");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio_util::sync::CancellationToken;
    use tower::util::ServiceExt;

    use super::service;

    #[tokio::test]
    async fn http_router_mounts_mcp_endpoint() {
        let known_roots = super::super::KnownRoots::default();
        let app = axum::Router::new()
            .nest_service("/mcp", service(CancellationToken::new(), known_roots));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/mcp")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
}
