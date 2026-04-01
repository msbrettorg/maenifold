use axum::{
    extract::Extension,
    http::header,
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/board"]
struct BoardAssets;

pub async fn index(Extension(port): Extension<u16>) -> Response {
    match BoardAssets::get("index.html") {
        Some(asset) => {
            let html = String::from_utf8_lossy(&asset.data);
            let injected = html.replacen(
                "<head>",
                &format!("<head>\n  <meta name=\"gate-port\" content=\"{port}\">"),
                1,
            );
            let mut response = Html(injected).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-cache"),
            );
            response
        }
        None => (
            axum::http::StatusCode::NOT_FOUND,
            "Gate board assets not found.",
        )
            .into_response(),
    }
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
