use super::{router, service};
use crate::api::routes::ApiState;
use crate::mcp::localhost_cors;
use crate::test_util::ConfigResetGuard;
use crate::{Config, GraphDatabase};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tempfile::TempDir;
use tower::ServiceExt;

fn test_state() -> (TempDir, ApiState, ConfigResetGuard) {
    let temp = TempDir::new().expect("temp dir");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    GraphDatabase::initialize_database(&config).expect("initialize database");
    (temp, ApiState::new(config, None), ConfigResetGuard)
}

fn test_router(state: ApiState) -> axum::Router {
    router(
        service(tokio_util::sync::CancellationToken::new(), None, None, None),
        state,
    )
    .layer(localhost_cors())
}

#[tokio::test]
#[serial_test::serial]
async fn router_serves_endophasia_frontend() {
    let (_temp, state, _guard) = test_state();
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/endophasia")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/html; charset=utf-8"
    );
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(body.contains("Endophasia"));
}

#[tokio::test]
#[serial_test::serial]
async fn router_serves_endophasia_communities_json() {
    let (_temp, state, _guard) = test_state();
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/endophasia/communities")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["communities"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

#[tokio::test]
#[serial_test::serial]
async fn router_allows_packaged_electron_null_origin() {
    let (_temp, state, _guard) = test_state();
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .header(header::ORIGIN, "null")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "null"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn router_allows_packaged_electron_file_origin() {
    let (_temp, state, _guard) = test_state();
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .header(header::ORIGIN, "file://")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "file://"
    );
}
