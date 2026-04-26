use super::{ApiState, api_routes};
use crate::{Config, db::GraphDatabase, test_util::ConfigResetGuard};
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
    crate::api::auth::generate_and_write_token(temp.path()).expect("write auth token");
    GraphDatabase::initialize_database(&config).expect("initialize database");
    (temp, ApiState::new(config, None), ConfigResetGuard)
}

fn auth_header(temp: &TempDir) -> String {
    let token_path = crate::api::auth::token_path(temp.path());
    let token = std::fs::read_to_string(token_path).expect("read auth token");
    format!("Bearer {}", token.trim())
}

#[tokio::test]
#[serial_test::serial]
async fn config_path_route_returns_bare_json_string() {
    let (temp, state, _guard) = test_state();
    let expected_path = temp.path().join("maenifold.json");
    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .uri("/api/config/path")
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
    assert_eq!(
        json,
        serde_json::Value::String(expected_path.to_string_lossy().to_string())
    );
}

#[tokio::test]
#[serial_test::serial]
async fn config_read_route_allows_missing_bearer_token() {
    let (_temp, state, _guard) = test_state();
    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial_test::serial]
async fn config_mutation_route_rejects_missing_bearer_token() {
    let (_temp, state, _guard) = test_state();
    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/reload")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial_test::serial]
async fn reload_config_route_reports_partial_runtime_reload() {
    let (temp, state, _guard) = test_state();
    std::fs::write(
        temp.path().join("maenifold.json"),
        r#"{
            "settings": {},
            "agents": {
                "local": {
                    "command": "/bin/echo hi"
                }
            }
        }"#,
    )
    .expect("write config");

    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/reload")
                .header(header::AUTHORIZATION, auth_header(&temp))
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

    assert_eq!(
        json,
        serde_json::json!({
            "config": {
                "agents": {
                    "local": {
                        "command": "/bin/echo hi"
                    }
                },
                "settings": serde_json::to_value(crate::acp_config::MaenifoldSettings::default())
                    .expect("default settings json")
            },
            "reloaded": ["configManagerState", "apiRuntimeSettings"],
            "requiresRestart": ["agentRuntimeBehavior"]
        })
    );
}
