use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rust_embed::Embed;

use crate::board_data::GateBoardData;
use crate::error::GateError;

#[derive(Debug, Clone)]
struct GateWebState {
    root: PathBuf,
}

#[derive(Embed)]
#[folder = "web-dist"]
struct WebAssets;

pub fn router(
    root: PathBuf,
    mcp_service: crate::mcp::http::GateHttpService,
) -> Router {
    let state = GateWebState { root };

    Router::new()
        .nest_service("/mcp", mcp_service)
        .route("/api/gate/tasks", get(list_tasks))
        .route("/api/gate/tasks/{task_id}", get(task_detail))
        .route("/", get(index))
        .fallback(static_handler)
        .with_state(state)
}

async fn list_tasks(
    State(state): State<GateWebState>,
) -> Result<Json<Vec<crate::board_data::BoardTaskSummary>>, AppError> {
    Ok(Json(GateBoardData::list(&state.root)?))
}

async fn task_detail(
    Path(task_id): Path<String>,
    State(state): State<GateWebState>,
) -> Result<Json<crate::board_data::BoardTaskDetail>, AppError> {
    Ok(Json(GateBoardData::detail(&state.root, &task_id)?))
}

async fn index() -> Response {
    asset_response("index.html")
}

async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return asset_response("index.html");
    }
    asset_response(path)
}

fn asset_response(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(content) => {
            let mime = if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".js") {
                "application/javascript"
            } else {
                "text/html; charset=utf-8"
            };
            let mut response = (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                content.data,
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            response
        }
        None => (StatusCode::NOT_FOUND, Html("gate web asset not found")).into_response(),
    }
}

struct AppError(GateError);

impl From<GateError> for AppError {
    fn from(value: GateError) -> Self {
        Self(value)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}

impl crate::mcp::GateMcpServer {
    pub async fn serve_web(port: u16) -> Result<(), GateError> {
        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| GateError::Cli(format!("HTTP bind failed on {bind_addr}: {e}")))?;
        let root = std::env::current_dir()
            .map_err(|e| GateError::Cli(format!("failed to resolve current directory: {e}")))?;
        let ct = tokio_util::sync::CancellationToken::new();
        let router = router(root, crate::mcp::http::service(ct.child_token()))
            .layer(crate::mcp::localhost_cors());

        eprintln!("gate listening on http://{bind_addr}/");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| GateError::Cli(format!("HTTP server error: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    use crate::mcp::http::service;

    use super::router;

    #[tokio::test]
    async fn web_router_serves_index() {
        let temp = temp_root();
        let app = router(temp.path().to_path_buf(), service(tokio_util::sync::CancellationToken::new()));
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).expect("request"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn web_router_serves_task_api() {
        let temp = temp_root();
        write_task(temp.path());
        let app = router(temp.path().to_path_buf(), service(tokio_util::sync::CancellationToken::new()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/gate/tasks")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    fn temp_root() -> TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("todo")).expect("todo");
        std::fs::create_dir_all(temp.path().join("done")).expect("done");
        temp
    }

    fn write_task(root: &std::path::Path) {
        let digest = authoritative_digest("sdd");
        std::fs::write(
            root.join("todo").join("T-1.1.md"),
            format!(
                "# T-1.1 — Example\n\n**Pipeline**: sdd\n**Pipeline Digest**: {digest}\n\nBody\n"
            ),
        )
        .expect("write task");
    }

    fn authoritative_digest(name: &str) -> String {
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(format!("{name}.json")),
        )
        .expect("read asset");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}
