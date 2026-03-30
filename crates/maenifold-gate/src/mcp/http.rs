use super::{GateHandler, TransportKind};
use axum::{
    Json,
    extract::State,
    routing::{get, post_service},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::{LocalSessionManager, SessionConfig},
};
use std::{sync::Arc, time::Duration};

async fn trace_data(
    State(trace_buffer): State<crate::trace::TraceBuffer>,
) -> Json<Vec<crate::trace::TraceEntry>> {
    Json(trace_buffer.snapshot())
}

impl super::GateMcpServer {
    pub async fn serve_http(port: u16) -> anyhow::Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let trace_buffer = crate::trace::TraceBuffer::new(20);
        let session_manager = Arc::new(LocalSessionManager {
            session_config: SessionConfig {
                keep_alive: Some(Duration::from_secs(300)),
                ..Default::default()
            },
            ..Default::default()
        });

        let root_service = StreamableHttpService::new(
            {
                let trace_buffer = trace_buffer.clone();
                move || {
                    Ok::<_, std::io::Error>(GateHandler::new(
                        TransportKind::Http,
                        Some(trace_buffer.clone()),
                    ))
                }
            },
            session_manager.clone(),
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );
        let mcp_service = StreamableHttpService::new(
            {
                let trace_buffer = trace_buffer.clone();
                move || {
                    Ok::<_, std::io::Error>(GateHandler::new(
                        TransportKind::Http,
                        Some(trace_buffer.clone()),
                    ))
                }
            },
            session_manager,
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );
        let board_service = StreamableHttpService::new(
            {
                let trace_buffer = trace_buffer.clone();
                move || {
                    Ok::<_, std::io::Error>(GateHandler::new(
                        TransportKind::Http,
                        Some(trace_buffer.clone()),
                    ))
                }
            },
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig {
                stateful_mode: false,
                json_response: true,
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

        let router = axum::Router::new()
            .route("/", get(crate::board::index).post_service(root_service))
            .route("/trace", get(crate::board::trace))
            .route("/trace/data", get(trace_data))
            .route("/board/api", post_service(board_service))
            .nest_service("/mcp", mcp_service)
            .layer(super::localhost_cors())
            .with_state(trace_buffer);

        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP bind failed on {bind_addr}: {e}"))?;

        eprintln!("gate board listening on http://{bind_addr}/");

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
