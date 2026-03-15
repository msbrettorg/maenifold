use crate::acp::config::AgentsConfig;
use crate::acp::relay::acp_routes;
use crate::api::routes::{ApiState, api_routes};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "$MAENIFOLD_WEB_DIST"]
#[allow(non_camel_case_types)]
struct WebAssets;

impl super::MaenifoldMcpServer {
    // T-17.3: Serve embedded SPA assets alongside the MCP HTTP transport.
    pub async fn serve_web(port: u16) -> Result<()> {
        // Reuse the same MCP setup as serve_http.
        use super::{MaenifoldHandler, TransportKind};
        use rmcp::transport::streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService,
            session::local::{LocalSessionManager, SessionConfig},
        };
        use std::{sync::Arc, time::Duration};
        let ct = tokio_util::sync::CancellationToken::new();
        let config = Config::current();
        super::initialize_runtime(&config)?;
        let config_root = config.maenifold_root.clone();
        let agents_config = AgentsConfig::load(&config_root);
        let api_state = ApiState::new(config.clone(), agents_config.clone());
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

        // FR-32-SEC: restrict CORS to localhost origins only.
        let cors = super::localhost_cors();

        let api = axum::Router::new()
            .merge(acp_routes(agents_config.clone()))
            .merge(api_routes(api_state))
            .fallback(static_handler);

        let router = axum::Router::new()
            .nest_service("/mcp", service)
            .merge(api)
            .layer(cors);

        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| {
                MaenifoldError::Message(format!("HTTP bind failed on {bind_addr}: {e}"))
            })?;

        eprintln!("maenifold web UI listening on http://{bind_addr}/");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| MaenifoldError::Message(format!("Web server error: {e}")))?;

        Ok(())
    }
}

async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try exact file match first.
    if let Some(content) = WebAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut response = (StatusCode::OK, content.data).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
        // Files under assets/ get immutable caching.
        if path.starts_with("assets/") {
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "max-age=31536000, immutable".parse().unwrap(),
            );
        }
        return response;
    }

    // SPA fallback: serve index.html for all non-API paths.
    match WebAssets::get("index.html") {
        Some(content) => {
            let mut response = Html(content.data).into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            response
        }
        None => (
            StatusCode::NOT_FOUND,
            "Web UI assets not found. Build the frontend first.",
        )
            .into_response(),
    }
}
