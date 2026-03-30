use axum::{
    http::header,
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/board"]
struct BoardAssets;

pub async fn index() -> Response {
    asset_response("index.html", "Gate board assets not found.")
}

pub async fn trace() -> Response {
    asset_response("trace.html", "Gate trace assets not found.")
}

fn asset_response(path: &str, not_found: &'static str) -> Response {
    match BoardAssets::get(path) {
        Some(asset) => {
            let mut response = Html(asset.data).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-cache"),
            );
            response
        }
        None => (axum::http::StatusCode::NOT_FOUND, not_found).into_response(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn embeds_index_html() {
        assert!(super::BoardAssets::get("index.html").is_some());
    }

    #[test]
    fn embeds_trace_html() {
        assert!(super::BoardAssets::get("trace.html").is_some());
    }
}
