use super::{MaenifoldHandler, TransportKind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig,
    StreamableHttpService,
    session::local::{LocalSessionManager, SessionConfig},
};
use std::{sync::Arc, time::Duration};
impl super::MaenifoldMcpServer {
    // T-17.2: RTM FR-23 (RTM-127, RTM-129, RTM-130) — expose Streamable HTTP MCP
    // endpoints with rmcp session management and browser-friendly CORS.
    pub async fn serve_http(port: u16) -> Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let config = Config::current();
        super::initialize_runtime(&config)?;
        let session_manager = Arc::new(LocalSessionManager {
            session_config: SessionConfig {
                keep_alive: Some(Duration::from_secs(300)),
                ..Default::default()
            },
            ..Default::default()
        });

        let service = StreamableHttpService::new(
            || {
                MaenifoldHandler::new(TransportKind::Http)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            },
            session_manager,
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

        let router = axum::Router::new()
            .nest_service("/mcp", service)
            // T-17.2: RTM FR-23 (RTM-130) — allow browser-based MCP clients to reach the
            // shared tool surface over HTTP.
            // FR-32-SEC: restrict CORS to localhost origins only.
            .layer(super::localhost_cors());

        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {bind_addr}: {e}"))
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
