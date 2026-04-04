use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};

use super::{GateMcpHandler, TransportKind};
use crate::error::GateError;

pub(crate) type GateHttpService = StreamableHttpService<GateMcpHandler, NeverSessionManager>;

pub(crate) fn service(
    cancellation_token: tokio_util::sync::CancellationToken,
    root: Arc<RwLock<Option<PathBuf>>>,
) -> GateHttpService {
    let session_manager = Arc::new(NeverSessionManager::default());
    let mut config = StreamableHttpServerConfig::default();
    config.stateful_mode = false;
    config.json_response = true;
    config.cancellation_token = cancellation_token;
    StreamableHttpService::new(
        move || {
            GateMcpHandler::new(TransportKind::Http, root.clone())
                .map_err(|e| std::io::Error::other(e.to_string()))
        },
        session_manager,
        config,
    )
}

impl super::GateMcpServer {
    pub async fn serve_http(
        port: u16,
        root: Arc<RwLock<Option<PathBuf>>>,
    ) -> Result<(), GateError> {
        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| GateError::Cli(format!("HTTP bind failed on {bind_addr}: {e}")))?;
        Self::serve_http_listener(tcp_listener, root).await
    }

    pub async fn serve_http_listener(
        tcp_listener: tokio::net::TcpListener,
        root: Arc<RwLock<Option<PathBuf>>>,
    ) -> Result<(), GateError> {
        let ct = tokio_util::sync::CancellationToken::new();
        let bind_addr = tcp_listener
            .local_addr()
            .map_err(|e| GateError::Cli(format!("HTTP listener address lookup failed: {e}")))?;
        let router = axum::Router::new()
            .nest_service("/mcp", service(ct.child_token(), root))
            .layer(super::localhost_cors());

        eprintln!("gate MCP HTTP server listening on http://{bind_addr}/mcp");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| GateError::Cli(format!("HTTP server error: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio_util::sync::CancellationToken;
    use tower::util::ServiceExt;

    use super::service;

    #[tokio::test]
    async fn http_router_mounts_mcp_endpoint() {
        let root = Arc::new(RwLock::new(None));
        let app = axum::Router::new().nest_service("/mcp", service(CancellationToken::new(), root));
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
