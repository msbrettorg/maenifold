use super::gateway_startup::{GatewayStartup, build_gateway_with_registry};
use super::{MaenifoldHandler, TransportKind};
use crate::api::routes::{ApiState, api_routes};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::mcp::PendingRequestRegistry;
use crate::mcp::gateway_runtime::GatewayState;
use crate::mcp::stdio_pool::StdioPool;
use axum::Router;
use axum::routing::{delete, get};
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
        .route(
            "/api/acp/session",
            get(crate::api::acp_relay::ws_handler).with_state(relay_state),
        )
        .route(
            "/api/acp/session/{session_id}",
            delete(crate::api::acp_relay::close_session_handler),
        )
}

// Stateless MCP service — no session affinity, no keepalive timeout, no
// stale-session errors. Every tool call is independent. Clients may send
// an `Mcp-Session-Id` header but the server ignores it and never rejects
// requests for session reasons. This eliminates the entire class of
// "session_expired" failures that occur when long-running AI workloads
// idle beyond the keepalive window.
#[allow(dead_code)]
pub(super) fn service(
    cancellation_token: tokio_util::sync::CancellationToken,
    config: Config,
    gateway: Option<Arc<GatewayState>>,
    gateway_diagnostics: Option<Arc<StdioPool>>,
) -> MaenifoldHttpService {
    let session_manager = Arc::new(NeverSessionManager::default());

    StreamableHttpService::new(
        move || {
            let mut handler =
                MaenifoldHandler::new_with_config(TransportKind::Http, config.clone())
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
            if let Some(gateway) = gateway.as_ref() {
                handler.set_gateway(gateway.clone());
            }
            if let Some(diagnostics) = gateway_diagnostics.as_ref() {
                handler.set_gateway_diagnostics(diagnostics.clone());
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

    /// Serve Streamable HTTP MCP on an already-bound listener.
    ///
    /// This is primarily public for integration tests that need an ephemeral
    /// port while exercising the real HTTP transport stack end-to-end.
    pub async fn serve_http_listener_with_shutdown(
        tcp_listener: tokio::net::TcpListener,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let config = Config::current();
        super::initialize_runtime(&config)?;
        let user_prompt_registry = Arc::new(PendingRequestRegistry::new());
        let gateway_startup =
            build_gateway_with_registry(&config, Arc::clone(&user_prompt_registry)).await?;
        let (gateway, pool_for_api) = match gateway_startup {
            GatewayStartup::Ready(gateway) => {
                let diagnostics_pool = Some(gateway.pool());
                (Some(gateway), diagnostics_pool)
            }
            GatewayStartup::Failed {
                gateway,
                diagnostics,
            } => {
                let diagnostics_pool = diagnostics.or_else(|| Some(gateway.pool()));
                (Some(gateway), diagnostics_pool)
            }
        };
        let router = router(
            service(
                shutdown_token.child_token(),
                config.clone(),
                gateway,
                pool_for_api.clone(),
            ),
            ApiState::with_user_prompt_registry(config.clone(), pool_for_api, user_prompt_registry),
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
    include!("http_tests.rs");
}
