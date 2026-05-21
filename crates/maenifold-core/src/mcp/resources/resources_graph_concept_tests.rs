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

fn setup_graph_concept_fixture() -> (TempDir, ConfigResetGuard) {
    let (tmp, guard) = setup_test_env();
    GraphDatabase::initialize_database(&Config::current()).expect("initialize graph database");
    let connection =
        db::open_connection(&Config::current().database_path).expect("open graph database");
    seed_concept(&connection, "maenifold", Some(7));
    seed_concept(&connection, "neighbor-concept", Some(7));
    seed_concept(&connection, "orphan-concept", None);
    connection
        .execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
             VALUES (?1, ?2, 3, ?3)",
            params!["maenifold", "neighbor-concept", "memory://fixtures/graph/maenifold.md"],
        )
        .expect("insert concept graph edge");
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
