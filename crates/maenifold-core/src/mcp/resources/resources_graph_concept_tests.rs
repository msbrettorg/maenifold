use super::resources_tests_common::*;
use crate::db::{self, GraphDatabase};
use rmcp::model::ResourceContents;
use rusqlite::params;
use serde_json::Value;

#[test]
#[serial]
fn graph_concept_includes_nullable_community_id_field() {
    let (_tmp, _guard) = setup_graph_concept_fixture();

    let with_community = read_payload("graph://concept/maenifold");
    assert_eq!(
        with_community.get("communityId").and_then(Value::as_i64),
        Some(7),
        "graph://concept/{{name}} must expose the integer communityId for clustered concepts",
    );

    let without_community = read_payload("graph://concept/orphan-concept");
    assert!(
        without_community.get("communityId").is_some(),
        "graph://concept/{{name}} must include communityId even when the concept has no community",
    );
    assert!(
        without_community.get("communityId").unwrap().is_null(),
        "concepts without a cluster must emit communityId: null instead of omitting the field",
    );
}

#[test]
#[serial]
fn graph_concept_neighbors_resource_pages_direct_relations() {
    let (_tmp, _guard) = setup_graph_concept_fixture();

    let first_page = read_payload("graph://concept/maenifold/neighbors?limit=2");
    assert_eq!(
        first_page.get("concept").and_then(Value::as_str),
        Some("maenifold")
    );
    assert_eq!(
        first_page.get("totalCount").and_then(Value::as_i64),
        Some(4)
    );
    assert_eq!(first_page.get("limit").and_then(Value::as_u64), Some(2));
    assert_eq!(first_page.get("cursor").and_then(Value::as_str), Some("0"));
    assert_eq!(
        first_page.get("nextCursor").and_then(Value::as_str),
        Some("2")
    );

    let neighbors = first_page
        .get("neighbors")
        .and_then(Value::as_array)
        .expect("neighbors array");
    assert_eq!(neighbors.len(), 2);
    assert_eq!(
        neighbors[0].get("name").and_then(Value::as_str),
        Some("high-weight")
    );
    assert_eq!(
        neighbors[1].get("name").and_then(Value::as_str),
        Some("middle-weight")
    );
    assert_eq!(
        neighbors[0].get("sourceFileCount").and_then(Value::as_u64),
        Some(1)
    );

    let second_page = read_payload("graph://concept/maenifold/neighbors?limit=2&cursor=2");
    assert!(second_page.get("nextCursor").unwrap().is_null());
    let second_neighbors = second_page
        .get("neighbors")
        .and_then(Value::as_array)
        .expect("second neighbors array");
    assert_eq!(second_neighbors.len(), 2);
    assert_eq!(
        second_neighbors[0].get("name").and_then(Value::as_str),
        Some("low-weight")
    );
    assert_eq!(
        second_neighbors[1].get("name").and_then(Value::as_str),
        Some("neighbor-concept")
    );
}

#[test]
#[serial]
fn graph_concept_neighbors_rejects_unbounded_limit() {
    let (_tmp, _guard) = setup_graph_concept_fixture();

    let error = handle_read_resource("graph://concept/maenifold/neighbors?limit=201")
        .expect_err("oversized neighbor page should be rejected");

    assert!(
        error.to_string().contains("limit must be <= 200"),
        "unexpected error: {error}"
    );
}

fn setup_graph_concept_fixture() -> (TempDir, ConfigResetGuard) {
    let (tmp, guard) = setup_test_env();
    GraphDatabase::initialize_database(&Config::current()).expect("initialize graph database");
    let connection =
        db::open_connection(&Config::current().database_path).expect("open graph database");
    seed_concept(&connection, "maenifold", Some(7));
    seed_concept(&connection, "neighbor-concept", Some(7));
    seed_concept(&connection, "high-weight", Some(7));
    seed_concept(&connection, "middle-weight", Some(7));
    seed_concept(&connection, "low-weight", Some(7));
    seed_concept(&connection, "orphan-concept", None);
    for (neighbor, weight) in [
        ("neighbor-concept", 3),
        ("high-weight", 9),
        ("middle-weight", 7),
        ("low-weight", 5),
    ] {
        connection
            .execute(
                "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    "maenifold",
                    neighbor,
                    weight,
                    "memory://fixtures/graph/maenifold.md"
                ],
            )
            .expect("insert concept graph edge");
    }
    (tmp, guard)
}

fn seed_concept(connection: &rusqlite::Connection, concept: &str, community_id: Option<i64>) {
    let timestamp = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count)
             VALUES (?1, ?2, 1)",
            params![concept, &timestamp],
        )
        .expect("insert concept");
    if let Some(community_id) = community_id {
        connection
            .execute(
                "INSERT INTO concept_communities (
                     concept_name,
                     community_id,
                     modularity,
                     resolution,
                     detected_at
                 ) VALUES (?1, ?2, 0.42, 1.0, ?3)",
                params![concept, community_id, &timestamp],
            )
            .expect("insert concept community");
    }
}

fn read_payload(uri: &str) -> Value {
    let result = handle_read_resource(uri).expect("graph concept resource should be readable");
    let Some(ResourceContents::TextResourceContents { text, .. }) = result.contents.first() else {
        panic!("expected text resource contents");
    };
    serde_json::from_str(text).expect("graph concept JSON")
}
