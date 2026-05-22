use super::{router, service};
use crate::api::routes::ApiState;
use crate::mcp::localhost_cors;
use crate::test_util::ConfigResetGuard;
use crate::{Config, GraphDatabase};
use axum::{
    body::Body,
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
async fn router_rejects_endophasia_frontend_route() {
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

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial_test::serial]
async fn router_rejects_endophasia_communities_route() {
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

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
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
