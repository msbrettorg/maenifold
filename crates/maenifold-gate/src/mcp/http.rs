use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};

use super::{GateMcpHandler, TransportKind};

pub(crate) type GateHttpService = StreamableHttpService<GateMcpHandler, NeverSessionManager>;

pub(crate) fn service(cancellation_token: tokio_util::sync::CancellationToken) -> GateHttpService {
    let session_manager = Arc::new(NeverSessionManager::default());
    StreamableHttpService::new(
        || {
            GateMcpHandler::new(TransportKind::Http)
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

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio_util::sync::CancellationToken;
    use tower::util::ServiceExt;

    use super::service;

    #[tokio::test]
    async fn http_router_mounts_mcp_endpoint() {
        let app = Router::new().nest_service("/mcp", service(CancellationToken::new()));
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
