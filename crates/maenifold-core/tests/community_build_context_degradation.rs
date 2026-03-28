// T-12.7: RTM NFR-1
#[allow(clippy::duplicate_mod)]
#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::community::detect_and_persist;
use maenifold_core::json::OutputFormat;
use serde_json::json;
use support::*;

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_GracefulDegradation_NoCommunityData (FR-13.10)
#[test]
fn build_context_graceful_degradation_no_community_data() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    insert_concept(&conn, "degrade-a");
    insert_concept(&conn, "degrade-b");
    insert_edge(&conn, "degrade-a", "degrade-b", 5);

    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM concept_communities", [], |r| r.get(0))
        .expect("count");
    assert_eq!(rows, 0, "concept_communities must be empty before the test");
    drop(conn);

    // Must not panic.
    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "degrade-a", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("BuildContext must not error when no community data");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let data = &parsed["data"];

    // CommunityId must be null.
    assert!(
        data["CommunityId"].is_null(),
        "FR-13.10: CommunityId must be null when no community data exists"
    );
    // CommunitySiblings must be empty.
    let siblings = data["CommunitySiblings"]
        .as_array()
        .expect("siblings array");
    assert!(
        siblings.is_empty(),
        "FR-13.10: CommunitySiblings must be empty when no community data exists"
    );
    // DirectRelations must still work.
    let direct = data["DirectRelations"].as_array().expect("direct array");
    assert!(
        !direct.is_empty(),
        "FR-13.10: DirectRelations must still be returned even without community data"
    );
    for rel in direct {
        assert!(
            rel["CommunityId"].is_null(),
            "FR-13.10: RelatedConcept '{}' CommunityId must be null",
            rel["Name"]
        );
    }
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_GracefulDegradation_ConceptNotInCommunities (FR-13.10)
#[test]
fn build_context_graceful_degradation_concept_not_in_communities() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Create a triangle and run community detection.
    for name in ["existing-a", "existing-b", "existing-c"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "existing-a", "existing-b", 10);
    insert_edge(&conn, "existing-b", "existing-c", 10);
    insert_edge(&conn, "existing-a", "existing-c", 10);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM concept_communities", [], |r| r.get(0))
        .expect("count");
    assert!(count > 0, "community data must exist for existing concepts");

    // Add orphan concept that was NOT part of community detection run.
    insert_concept(&conn, "orphan-concept");
    insert_edge(&conn, "orphan-concept", "existing-a", 5);

    let orphan_in_comm: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM concept_communities WHERE concept_name = 'orphan-concept'",
            [],
            |r| r.get(0),
        )
        .expect("check");
    assert!(
        !orphan_in_comm,
        "orphan-concept must not be in concept_communities"
    );
    drop(conn);

    // Must not error.
    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "orphan-concept", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("BuildContext must not error for orphan concept");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let data = &parsed["data"];

    assert!(
        data["CommunityId"].is_null(),
        "FR-13.10: orphan concept CommunityId must be null"
    );
    let siblings = data["CommunitySiblings"].as_array().expect("siblings");
    assert!(
        siblings.is_empty(),
        "FR-13.10: CommunitySiblings must be empty for orphan concept"
    );

    let empty_vec2 = vec![];
    let direct_names: Vec<&str> = data["DirectRelations"]
        .as_array()
        .unwrap_or(&empty_vec2)
        .iter()
        .filter_map(|r| r["Name"].as_str())
        .collect();
    assert!(
        direct_names.contains(&"existing-a"),
        "FR-13.10: existing-a must still appear in DirectRelations for orphan"
    );

    // Rust behaviour: when the query concept has no community assignment,
    // include_community_data = false, so ALL DirectRelation CommunityIds are null.
    // (build_context.rs:61, formatter.rs:28–35)
    let empty_vec3 = vec![];
    let existing_a_rel = data["DirectRelations"]
        .as_array()
        .unwrap_or(&empty_vec3)
        .iter()
        .find(|r| r["Name"] == "existing-a")
        .expect("existing-a relation");
    assert!(
        existing_a_rel["CommunityId"].is_null(),
        "FR-13.10: when query concept is not in a community, relation CommunityIds \
         must also be null (include_community_data=false path)"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_NullCommunityId_DoesNotCrash (FR-16.10)
#[test]
fn build_context_mixed_community_state_in_direct_relations_is_correct() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Step 1: triangle with community data.
    for name in ["null-comm-a", "null-comm-b", "null-comm-c"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "null-comm-a", "null-comm-b", 10);
    insert_edge(&conn, "null-comm-b", "null-comm-c", 10);
    insert_edge(&conn, "null-comm-a", "null-comm-c", 10);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    // Step 2: add orphan with edges — no re-run.
    let conn = open_conn(&env.db_path);
    insert_concept(&conn, "null-comm-orphan");
    insert_edge(&conn, "null-comm-a", "null-comm-orphan", 5);
    insert_edge(&conn, "null-comm-b", "null-comm-orphan", 3);

    let orphan_has_comm: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM concept_communities WHERE concept_name = 'null-comm-orphan'",
            [],
            |r| r.get(0),
        )
        .expect("check orphan");
    assert!(
        !orphan_has_comm,
        "null-comm-orphan must not have community assignment"
    );
    drop(conn);

    // Step 3: BuildContext on null-comm-a — has a direct relation to the orphan.
    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "null-comm-a", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("FR-16.10: BuildContext must not crash when related concept has null community_id");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let data = &parsed["data"];

    // Orphan must appear with null CommunityId.
    let direct = data["DirectRelations"].as_array().expect("direct");
    let orphan_rel = direct.iter().find(|r| r["Name"] == "null-comm-orphan");
    assert!(
        orphan_rel.is_some(),
        "null-comm-orphan must appear in DirectRelations"
    );
    assert!(
        orphan_rel.unwrap()["CommunityId"].is_null(),
        "FR-16.10: orphan concept's CommunityId must be null"
    );

    // null-comm-b must still have its community assigned.
    let b_rel = direct.iter().find(|r| r["Name"] == "null-comm-b");
    assert!(
        b_rel.is_some(),
        "null-comm-b must appear in DirectRelations"
    );
    assert!(
        !b_rel.unwrap()["CommunityId"].is_null(),
        "null-comm-b must have non-null CommunityId (was assigned before orphan was added)"
    );
}
