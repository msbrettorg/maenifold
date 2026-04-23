use super::{ApiState, api_routes};
use crate::{
    Config,
    db::{self, GraphDatabase},
    test_util::ConfigResetGuard,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use rusqlite::params;
use tempfile::TempDir;
use tower::ServiceExt;

fn test_state() -> (TempDir, ApiState, ConfigResetGuard) {
    let temp = TempDir::new().expect("temp dir");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    GraphDatabase::initialize_database(&config).expect("initialize database");
    (temp, ApiState::new(config), ConfigResetGuard)
}

fn seed_graph_community_fixture(state: &ApiState) {
    let config = state.config();
    let connection = db::open_connection(&config.database_path).expect("open connection");

    for (file_path, created) in [
        ("memory/community-alpha.md", "2024-01-01T00:00:00Z"),
        ("memory/community-beta.md", "2024-01-02T00:00:00Z"),
        ("memory/community-gamma.md", "2024-01-03T00:00:00Z"),
        ("memory/community-delta.md", "2024-01-04T00:00:00Z"),
        ("memory/community-epsilon.md", "2024-01-05T00:00:00Z"),
        ("memory/community-zeta.md", "2024-01-06T00:00:00Z"),
    ] {
        connection
            .execute(
                "INSERT INTO file_content (
                    file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed
                ) VALUES (?1, ?2, '', '', ?3, 'indexed', '', 0, ?3, ?3)",
                params![file_path, file_path, created],
            )
            .expect("insert file_content");
    }

    for (concept_name, occurrence_count, source_file, community_id) in [
        ("alpha", 9_i64, "memory/community-alpha.md", 1_i64),
        ("beta", 7, "memory/community-beta.md", 1),
        ("gamma", 5, "memory/community-gamma.md", 1),
        ("delta", 8, "memory/community-delta.md", 2),
        ("epsilon", 6, "memory/community-epsilon.md", 2),
        ("zeta", 4, "memory/community-zeta.md", 2),
    ] {
        connection
            .execute(
                "INSERT INTO concepts (concept_name, first_seen, occurrence_count)
                 VALUES (?1, '2024-01-01T00:00:00Z', ?2)",
                params![concept_name, occurrence_count],
            )
            .expect("insert concept");
        connection
            .execute(
                "INSERT INTO concept_mentions (concept_name, source_file, mention_count)
                 VALUES (?1, ?2, 1)",
                params![concept_name, source_file],
            )
            .expect("insert concept mention");
        connection
            .execute(
                "INSERT INTO concept_communities (
                    concept_name, community_id, modularity, resolution, detected_at
                ) VALUES (?1, ?2, 0.42, 1.0, '2024-01-01T00:00:00Z')",
                params![concept_name, community_id],
            )
            .expect("insert concept community");
    }

    for (concept_a, concept_b, weight) in [
        ("alpha", "beta", 6_i64),
        ("alpha", "gamma", 4),
        ("beta", "gamma", 3),
        ("delta", "epsilon", 5),
        ("delta", "zeta", 4),
        ("epsilon", "zeta", 2),
        ("alpha", "delta", 6),
    ] {
        connection
            .execute(
                "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                 VALUES (?1, ?2, ?3, '[]')",
                params![concept_a, concept_b, weight],
            )
            .expect("insert concept graph edge");
    }
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

#[tokio::test]
#[serial_test::serial]
async fn api_graph_communities_endpoint_returns_seeded_community_graph() {
    let (_temp, state, _guard) = test_state();
    seed_graph_community_fixture(&state);

    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .uri("/api/graph/communities")
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
            "nodes": [
                { "id": 1, "anchor": "alpha", "size": 3 },
                { "id": 2, "anchor": "delta", "size": 3 }
            ],
            "edges": [
                { "source": 1, "target": 2, "weight": 6 }
            ]
        })
    );
}

#[tokio::test]
#[serial_test::serial]
async fn api_graph_community_concepts_endpoint_returns_seeded_concept_graph() {
    let (_temp, state, _guard) = test_state();
    seed_graph_community_fixture(&state);

    let response = api_routes(state)
        .oneshot(
            Request::builder()
                .uri("/api/graph/communities/1/concepts")
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
            "communityId": 1,
            "anchor": "alpha",
            "nodes": [
                { "name": "alpha", "occurrences": 9 },
                { "name": "beta", "occurrences": 7 },
                { "name": "gamma", "occurrences": 5 }
            ],
            "edges": [
                { "source": "alpha", "target": "beta", "weight": 6 },
                { "source": "alpha", "target": "gamma", "weight": 4 },
                { "source": "beta", "target": "gamma", "weight": 3 }
            ]
        })
    );
}
