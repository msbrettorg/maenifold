pub(crate) fn localhost_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
    let origin_predicate = AllowOrigin::predicate(
        |origin: &axum::http::HeaderValue, _parts: &axum::http::request::Parts| {
            let Ok(s) = origin.to_str() else {
                return false;
            };
            s == "null"
                || s == "file://"
                || s == "http://localhost"
                || s.starts_with("http://localhost:")
                || s == "http://127.0.0.1"
                || s.starts_with("http://127.0.0.1:")
                || s == "http://[::1]"
                || s.starts_with("http://[::1]:")
                // Tauri 2 WebView2 (Windows) issues fetches from
                // http://tauri.localhost; macOS uses tauri://localhost.
                // Both are first-party app origins and must be allowed
                // so the bundled UI can talk to the local /mcp endpoint.
                || s == "http://tauri.localhost"
                || s == "https://tauri.localhost"
                || s == "tauri://localhost"
        },
    );
    CorsLayer::new()
        .allow_origin(origin_predicate)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers([axum::http::HeaderName::from_static("mcp-session-id")])
}
