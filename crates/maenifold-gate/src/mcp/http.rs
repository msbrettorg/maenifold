use super::{GateHandler, TransportKind};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::{LocalSessionManager, SessionConfig},
};
use std::{sync::Arc, time::Duration};

impl super::GateMcpServer {
    pub async fn serve_http(port: u16) -> anyhow::Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let session_manager = Arc::new(LocalSessionManager {
            session_config: SessionConfig {
                keep_alive: Some(Duration::from_secs(300)),
                ..Default::default()
            },
            ..Default::default()
        });

        let service = StreamableHttpService::new(
            || Ok::<_, std::io::Error>(GateHandler::new(TransportKind::Http)),
            session_manager,
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

        let router = axum::Router::new()
            .nest_service("/mcp", service)
            .layer(super::localhost_cors());

        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP bind failed on {bind_addr}: {e}"))?;

        eprintln!("gate MCP HTTP server listening on http://{bind_addr}/mcp");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))?;

        Ok(())
    }
}
