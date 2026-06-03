use super::gateway_startup::{GatewayStartup, build_gateway_with_registry};
use super::{MaenifoldHandler, TransportKind};
use crate::api::routes::{ApiState, ApiStateSharedConfig, api_routes};
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

fn mcp_router_with_service(service: MaenifoldHttpService) -> Router {
    Router::new()
        .nest_service("/mcp", service)
        .layer(super::localhost_cors())
}

fn local_api_router_with_state(state: ApiState) -> Router {
    let relay_state = state.clone();
    api_routes(state)
        .route(
            "/api/acp/session",
            get(crate::api::acp_relay::ws_handler).with_state(relay_state),
        )
        .route(
            "/api/acp/session/{session_id}",
            delete(crate::api::acp_relay::close_session_handler),
        )
        .layer(super::localhost_cors())
}

pub fn mcp_router(config: &Config) -> Router {
    mcp_router_with_service(service(
        tokio_util::sync::CancellationToken::new(),
        config.clone(),
        None,
        None,
    ))
}

pub fn local_api_router(config: &Config) -> Router {
    local_api_router_with_state(ApiState::new(config.clone(), None))
}

fn http_server_config(
    cancellation_token: tokio_util::sync::CancellationToken,
    config: &Config,
) -> StreamableHttpServerConfig {
    let mut server_config = StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_cancellation_token(cancellation_token);
    let allowed_hosts = config.mcp_allowed_hosts();
    if !allowed_hosts.is_empty() {
        server_config = server_config.with_allowed_hosts(allowed_hosts);
    }
    let allowed_origins = config.mcp_allowed_origins();
    if !allowed_origins.is_empty() {
        server_config = server_config.with_allowed_origins(allowed_origins);
    }
    server_config
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
    let service_config = http_server_config(cancellation_token, &config);
    let handler_config = config.clone();

    StreamableHttpService::new(
        move || {
            let mut handler =
                MaenifoldHandler::new_with_config(TransportKind::Http, handler_config.clone())
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
        service_config,
    )
}

async fn build_http_stack(
    config: Config,
    shutdown_token: tokio_util::sync::CancellationToken,
) -> Result<(MaenifoldHttpService, ApiState)> {
    super::initialize_runtime(&config)?;
    let shared_config = ApiStateSharedConfig::from(config.clone());
    let user_prompt_registry = Arc::new(PendingRequestRegistry::new());
    let gateway_startup =
        build_gateway_with_registry(shared_config.clone(), Arc::clone(&user_prompt_registry))
            .await?;
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

    Ok((
        service(
            shutdown_token.child_token(),
            config,
            gateway,
            pool_for_api.clone(),
        ),
        ApiState::with_user_prompt_registry(shared_config, pool_for_api, user_prompt_registry),
    ))
}

async fn serve_router_listener_with_shutdown(
    tcp_listener: tokio::net::TcpListener,
    router: Router,
    shutdown_token: tokio_util::sync::CancellationToken,
    endpoint: String,
    label: &'static str,
) -> Result<()> {
    info!(endpoint = %endpoint, listener = label, "maenifold HTTP server bound");

    let graceful_shutdown_token = shutdown_token.clone();
    let shutdown_endpoint = endpoint.clone();
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = graceful_shutdown_token.cancelled() => {}
            }
            info!(endpoint = %shutdown_endpoint, listener = label, "maenifold HTTP server shutting down");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        })
        .await
        .map_err(|e| MaenifoldError::Message(format!("HTTP server error: {e}")))?;

    info!(endpoint = %endpoint, listener = label, "maenifold HTTP server shutdown complete");
    Ok(())
}

impl super::MaenifoldMcpServer {
    pub async fn serve_split(mcp_bind: &str, mcp_port: u16, local_api_port: u16) -> Result<()> {
        Self::serve_split_with_shutdown(
            mcp_bind,
            mcp_port,
            local_api_port,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
    }

    pub async fn serve_split_with_shutdown(
        mcp_bind: &str,
        mcp_port: u16,
        local_api_port: u16,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let mcp_bind_addr = format!("{mcp_bind}:{mcp_port}");
        let local_api_bind_addr = format!("127.0.0.1:{local_api_port}");
        let mcp_listener = tokio::net::TcpListener::bind(&mcp_bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {mcp_bind_addr}: {e}"))
            })?;
        let local_api_listener = tokio::net::TcpListener::bind(&local_api_bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {local_api_bind_addr}: {e}"))
            })?;
        let config = Config::current();
        let (service, state) = build_http_stack(config, shutdown_token.clone()).await?;
        let mcp_addr = mcp_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;
        let local_api_addr = local_api_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;
        let mcp_task = tokio::spawn(serve_router_listener_with_shutdown(
            mcp_listener,
            mcp_router_with_service(service),
            shutdown_token.clone(),
            format!("http://{mcp_addr}/mcp"),
            "mcp",
        ));
        let local_api_task = tokio::spawn(serve_router_listener_with_shutdown(
            local_api_listener,
            local_api_router_with_state(state),
            shutdown_token,
            format!("http://{local_api_addr}/api/health"),
            "local-api",
        ));
        let (mcp_result, local_api_result) = tokio::join!(mcp_task, local_api_task);
        mcp_result
            .map_err(|e| MaenifoldError::Message(format!("MCP listener join failed: {e}")))??;
        local_api_result.map_err(|e| {
            MaenifoldError::Message(format!("Local API listener join failed: {e}"))
        })??;
        Ok(())
    }

    /// Serve the MCP-only router on an already-bound listener.
    ///
    /// Public for integration tests that need an ephemeral port while exercising
    /// the real HTTP transport stack end-to-end. Production callers use
    /// `serve_split`, which binds the local-API listener on its own loopback port.
    pub async fn serve_http_listener_with_shutdown(
        tcp_listener: tokio::net::TcpListener,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let config = Config::current();
        let bind_addr = tcp_listener.local_addr().map_err(|e| {
            MaenifoldError::Message(format!("HTTP listener address lookup failed: {e}"))
        })?;
        let (service, _state) = build_http_stack(config, shutdown_token.clone()).await?;
        serve_router_listener_with_shutdown(
            tcp_listener,
            mcp_router_with_service(service),
            shutdown_token,
            format!("http://{bind_addr}/mcp"),
            "mcp",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    include!("http_tests.rs");
}
