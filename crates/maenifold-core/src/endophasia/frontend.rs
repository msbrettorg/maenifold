use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/endophasia/frontend/"]
struct EndophasiaFrontendAssets;

pub(crate) fn index_html_response() -> Response {
    match EndophasiaFrontendAssets::get("index.html") {
        Some(content) => {
            let mut response = (StatusCode::OK, content.data).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("text/html; charset=utf-8"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-cache"),
            );
            response
        }
        None => (
            StatusCode::NOT_FOUND,
            "Endophasia frontend asset not found.",
        )
            .into_response(),
    }
}
