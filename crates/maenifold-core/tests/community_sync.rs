// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::json::OutputFormat;
use serde_json::json;
use support::*;

// ─────────────────────────────────────────────────────────────────────────────
// §4  Integration tests — mirrors the integration section of CommunityDetectionTests.cs
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// Mirrors: Sync_ProducesCommunityAssignments (FR-13.4)
//
// This is the only test exercising community detection through the live Sync
// tool via ToolRegistry. All other community tests call detect_and_persist()
// directly. Assertions verify partition *structure*, not just presence.
#[test]
fn sync_produces_community_assignments() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    let registry = ToolRegistry::global();
    // Write memory files with overlapping WikiLinks so Sync builds a graph.
    // Cluster 1: a–b–c triangle (a–b co-occurs in two files → strong edge).
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

    // Cluster 2: isolated d–e pair.
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

    // FR-13.4: concept_communities must be populated.
    let rows = community_row_count(&conn);
    assert!(
        rows > 0,
        "FR-13.4: Sync must populate concept_communities (got 0 rows)"
    );

    // FR-13.4: Louvain must produce at least 2 distinct communities for two
    // disconnected clusters.
    let distinct = distinct_community_count(&conn);
    assert!(
        distinct >= 2,
        "FR-13.4: expected >= 2 distinct communities for two disconnected clusters, got {distinct}"
    );

    // FR-13.4: Concepts from the same cluster must share a community ID.
    let id_a = get_community_id(&conn, "sync-comm-a")
        .expect("sync-comm-a must have community assignment after Sync");
    let id_b = get_community_id(&conn, "sync-comm-b")
        .expect("sync-comm-b must have community assignment after Sync");
    assert_eq!(
        id_a, id_b,
        "FR-13.4: sync-comm-a and sync-comm-b must be in the same community (triangle cluster)"
    );

    let id_d = get_community_id(&conn, "sync-comm-d")
        .expect("sync-comm-d must have community assignment after Sync");
    let id_e = get_community_id(&conn, "sync-comm-e")
        .expect("sync-comm-e must have community assignment after Sync");
    assert_eq!(
        id_d, id_e,
        "FR-13.4: sync-comm-d and sync-comm-e must be in the same community (isolated pair)"
    );

    // FR-13.4: Concepts from different clusters must be in different communities.
    assert_ne!(
        id_a, id_d,
        "FR-13.4: sync-comm-a and sync-comm-d must be in different communities (disconnected clusters)"
    );
}
