use super::{MaenifoldHandler, TransportKind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::any,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::{LocalSessionManager, SessionConfig},
};
use std::{sync::Arc, time::Duration};

pub(super) type MaenifoldHttpService = StreamableHttpService<MaenifoldHandler, LocalSessionManager>;

const MCP_SESSION_ID_HEADER: &str = "mcp-session-id";

// T-74.2: MCP spec (2025-03-26) says servers MUST return HTTP 404 for
// unknown/terminated sessions.  Clients MUST re-initialize by sending a new
// `initialize` request without the Mcp-Session-Id header.  rmcp's built-in
// message is a bare plain-text string; we rewrite it to a JSON body with
// clear recovery instructions so clients can act programmatically.
const STALE_SESSION_JSON: &str = concat!(
    r#"{"error":"session_expired","message":"#,
    r#""MCP session not found. The session may have expired or the server was"#,
    r#" restarted. Re-initialize the session by sending a new initialize"#,
    r#" request without the Mcp-Session-Id header to create a fresh session."}"#,
);

const STALE_SESSION_BODY_LIMIT: usize = 8 * 1024;

pub(super) fn router(service: MaenifoldHttpService) -> Router {
    Router::new()
        .route("/mcp", any(handle_request))
        .with_state(service)
}

pub(super) fn service(
    cancellation_token: tokio_util::sync::CancellationToken,
) -> MaenifoldHttpService {
    let session_manager = Arc::new(LocalSessionManager {
        session_config: SessionConfig {
            keep_alive: Some(Duration::from_secs(300)),
            ..Default::default()
        },
        ..Default::default()
    });

    StreamableHttpService::new(
        || {
            MaenifoldHandler::new(TransportKind::Http)
                .map_err(|e| std::io::Error::other(e.to_string()))
        },
        session_manager,
        StreamableHttpServerConfig {
            cancellation_token,
            ..Default::default()
        },
    )
}

pub(super) async fn handle_request(
    State(service): State<MaenifoldHttpService>,
    request: Request,
) -> Response {
    let session_id = request
        .headers()
        .get(MCP_SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let response = service.handle(request).await.map(Body::new);
    rewrite_stale_session_response(session_id, response).await
}

// T-74.2: If the client sent an Mcp-Session-Id and rmcp returned 404
// (session expired / server restarted), replace the plain-text body
// with structured JSON carrying re-initialization instructions.
pub(super) async fn rewrite_stale_session_response(
    session_id: Option<String>,
    response: axum::http::Response<Body>,
) -> Response {
    let Some(session_id) = session_id else {
        return response;
    };

    if response.status() != StatusCode::NOT_FOUND {
        return response;
    }

    let (parts, body) = response.into_parts();
    let body_bytes = match to_bytes(body, STALE_SESSION_BODY_LIMIT).await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(
                session_id = %session_id,
                %error,
                "Failed to inspect 404 response body for stale-session rewrite"
            );
            return axum::http::Response::from_parts(parts, Body::empty());
        }
    };

    let body_text = String::from_utf8_lossy(&body_bytes);
    let body_lower = body_text.to_ascii_lowercase();
    let looks_like_stale_session =
        body_lower.contains("not found") || body_lower.contains("session");

    if looks_like_stale_session {
        tracing::warn!(
            session_id = %session_id,
            "Stale Mcp-Session-Id — session expired or server restarted"
        );
        return (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            STALE_SESSION_JSON,
        )
            .into_response();
    }

    axum::http::Response::from_parts(parts, Body::from(body_bytes))
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
