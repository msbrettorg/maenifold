// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::community::detect_and_persist;
use maenifold_core::json::OutputFormat;
use serde_json::json;
use support::*;

// ─────────────────────────────────────────────────────────────────────────────
// §4  Integration tests — mirrors the integration section of CommunityDetectionTests.cs
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// Mirrors: Sync_ProducesCommunityAssignments (FR-13.4)
#[test]
fn sync_produces_community_assignments() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    let registry = ToolRegistry::global();
    // Write memory files with overlapping WikiLinks so Sync builds a graph.
    registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": "Community Sync Note ABC",
                "content": "Discusses [[sync-comm-a]] and [[sync-comm-b]] and [[sync-comm-c]].",
                "folder": "tests/community-sync"
            }),
            OutputFormat::Json,
        )
        .expect("write abc");

    registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": "Community Sync Note AB",
                "content": "More about [[sync-comm-a]] and [[sync-comm-b]].",
                "folder": "tests/community-sync"
            }),
            OutputFormat::Json,
        )
        .expect("write ab");

    registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": "Community Sync Note DE",
                "content": "About [[sync-comm-d]] and [[sync-comm-e]].",
                "folder": "tests/community-sync"
            }),
            OutputFormat::Json,
        )
        .expect("write de");

    registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync");

    let conn = open_conn(&env.db_path);
    let rows = community_row_count(&conn);
    assert!(
        rows > 0,
        "Sync must populate concept_communities (got 0 rows)"
    );

    let names: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT concept_name FROM concept_communities")
            .expect("stmt");
        stmt.query_map([], |r| r.get(0))
            .expect("map")
            .collect::<Result<_, _>>()
            .expect("collect")
    };
    assert!(
        names.contains(&"sync-comm-a".to_string()),
        "sync-comm-a must have community assignment after Sync"
    );
    assert!(
        names.contains(&"sync-comm-d".to_string()),
        "sync-comm-d must have community assignment after Sync"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_IncludesCommunityIdOnRelatedConcepts (FR-13.6)
#[test]
fn build_context_includes_community_id_on_related_concepts() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    insert_concept(&conn, "ctx-alpha");
    insert_concept(&conn, "ctx-beta");
    insert_concept(&conn, "ctx-gamma");
    insert_edge(&conn, "ctx-alpha", "ctx-beta", 10);
    insert_edge(&conn, "ctx-beta", "ctx-gamma", 10);
    insert_edge(&conn, "ctx-alpha", "ctx-gamma", 10);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    let persisted = community_row_count(&conn);
    assert!(
        persisted > 0,
        "communities must be persisted before BuildContext"
    );
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "ctx-alpha", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let data = &parsed["data"];

    // Query concept must have a CommunityId.
    let query_community_id = &data["CommunityId"];
    assert!(
        !query_community_id.is_null(),
        "query concept ctx-alpha must have non-null CommunityId when community data exists"
    );

    // Direct relations must have CommunityId populated.
    let direct = data["DirectRelations"]
        .as_array()
        .expect("DirectRelations array");
    assert!(!direct.is_empty(), "ctx-alpha must have direct relations");
    for rel in direct {
        assert!(
            !rel["CommunityId"].is_null(),
            "FR-13.6: DirectRelation '{}' must have non-null CommunityId",
            rel["Name"]
        );
    }
}
