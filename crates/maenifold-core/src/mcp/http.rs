use super::{MaenifoldHandler, TransportKind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::never::NeverSessionManager,
};
use std::sync::Arc;

pub(super) type MaenifoldHttpService =
    StreamableHttpService<MaenifoldHandler, NeverSessionManager>;

pub(super) fn router(service: MaenifoldHttpService) -> Router {
    Router::new().nest_service("/mcp", service)
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
        StreamableHttpServerConfig {
            stateful_mode: false,
            json_response: true,
            cancellation_token,
            ..Default::default()
        },
    )
}

impl super::MaenifoldMcpServer {
    // T-17.2: RTM FR-23 (RTM-127, RTM-129, RTM-130) — expose Streamable HTTP MCP
    // endpoints with rmcp session management and browser-friendly CORS.
    pub async fn serve_http(port: u16) -> Result<()> {
        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {bind_addr}: {e}"))
            })?;

        Self::serve_http_listener(tcp_listener).await
    }

    pub async fn serve_http_listener(tcp_listener: tokio::net::TcpListener) -> Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let config = Config::current();
        super::initialize_runtime(&config)?;
        let router = router(service(ct.child_token()))
            // T-17.2: RTM FR-23 (RTM-130) — allow browser-based MCP clients to reach the
            // shared tool surface over HTTP.
            // FR-32-SEC: restrict CORS to localhost origins only.
            .layer(super::localhost_cors());
        let bind_addr = tcp_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;

        eprintln!("maenifold MCP HTTP server listening on http://{bind_addr}/mcp");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| MaenifoldError::Message(format!("HTTP server error: {e}")))?;

        Ok(())
    }
}
