use super::{GateHandler, TransportKind};
use axum::{
    Json,
    extract::State,
    routing::get,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    trace_buffer: crate::trace::TraceBuffer,
}

async fn trace_data(State(state): State<AppState>) -> Json<Vec<crate::trace::TraceEntry>> {
    Json(state.trace_buffer.snapshot())
}

impl super::GateMcpServer {
    pub async fn serve_http(port: u16) -> anyhow::Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let trace_buffer = crate::trace::TraceBuffer::new(20);
        let app_state = AppState {
            trace_buffer: trace_buffer.clone(),
        };
        let session_manager = Arc::new(NeverSessionManager::default());
        // Serve both HTTP entrypoints in stateless JSON-response mode so daemon restarts do not
        // reject clients that resend a stale `Mcp-Session-Id`. In this mode MCP remains POST-only;
        // clients should continue sending `MCP-Protocol-Version`, but the server no longer requires
        // session affinity via `Mcp-Session-Id` and responds with JSON instead of SSE setup.

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
                stateful_mode: false,
                json_response: true,
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
            .nest_service("/mcp", mcp_service)
            .layer(super::localhost_cors())
            .layer(axum::Extension(port))
            .with_state(app_state);

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
