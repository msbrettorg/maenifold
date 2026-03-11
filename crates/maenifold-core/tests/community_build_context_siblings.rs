// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::community::detect_and_persist;
use maenifold_core::json::OutputFormat;
use rusqlite::params;
use serde_json::json;
use support::*;

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_ReturnsCommunitySiblings (FR-13.7, FR-13.8)
#[test]
fn build_context_returns_community_siblings() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // sib-query and sib-target share 4 strong neighbors but have no direct edge.
    for name in [
        "sib-query",
        "sib-target",
        "sib-n1",
        "sib-n2",
        "sib-n3",
        "sib-n4",
    ] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "sib-query", "sib-n1", 10);
    insert_edge(&conn, "sib-query", "sib-n2", 10);
    insert_edge(&conn, "sib-query", "sib-n3", 10);
    insert_edge(&conn, "sib-query", "sib-n4", 10);
    insert_edge(&conn, "sib-target", "sib-n1", 10);
    insert_edge(&conn, "sib-target", "sib-n2", 10);
    insert_edge(&conn, "sib-target", "sib-n3", 10);
    insert_edge(&conn, "sib-target", "sib-n4", 10);
    // Internal edges among neighbors for cluster tightness.
    insert_edge(&conn, "sib-n1", "sib-n2", 10);
    insert_edge(&conn, "sib-n1", "sib-n3", 10);
    insert_edge(&conn, "sib-n1", "sib-n4", 10);
    insert_edge(&conn, "sib-n2", "sib-n3", 10);
    insert_edge(&conn, "sib-n2", "sib-n4", 10);
    insert_edge(&conn, "sib-n3", "sib-n4", 10);
    drop(conn);

    let comm_count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    assert!(comm_count > 0, "communities must be detected");

    let conn = open_conn(&env.db_path);
    let query_comm = get_community_id(&conn, "sib-query").expect("sib-query community");
    let target_comm = get_community_id(&conn, "sib-target").expect("sib-target community");
    assert_eq!(
        query_comm, target_comm,
        "sib-query and sib-target must be in the same community"
    );
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "sib-query", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let data = &parsed["data"];

    // sib-target must NOT be in DirectRelations (no direct edge to sib-query).
    let empty_vec = vec![];
    let direct_names: Vec<&str> = data["DirectRelations"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|r| r["Name"].as_str())
        .collect();
    assert!(
        !direct_names.contains(&"sib-target"),
        "sib-target must not be a direct relation (no direct edge)"
    );

    // FR-13.7: sib-target must appear in CommunitySiblings.
    let siblings = data["CommunitySiblings"]
        .as_array()
        .expect("CommunitySiblings");
    let sibling_names: Vec<&str> = siblings.iter().filter_map(|s| s["Name"].as_str()).collect();
    assert!(
        sibling_names.contains(&"sib-target"),
        "FR-13.7: sib-target must appear in CommunitySiblings (same community, shared neighbors)"
    );

    // FR-13.8: Scoring fields must be populated.
    let target_sib = siblings.iter().find(|s| s["Name"] == "sib-target").unwrap();
    let overlap = target_sib["NormalizedOverlap"].as_f64().unwrap_or(0.0);
    assert!(
        overlap >= 0.4,
        "FR-13.8: NormalizedOverlap must be >= 0.4, got {overlap}"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_SiblingThresholdsFilter (FR-13.9)
#[test]
fn build_context_sibling_thresholds_filter() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    for name in [
        "thresh-query",
        "thresh-weak",
        "thresh-strong",
        "thresh-n1",
        "thresh-n2",
        "thresh-n3",
        "thresh-n4",
    ] {
        insert_concept(&conn, name);
    }

    insert_edge(&conn, "thresh-query", "thresh-n1", 10);
    insert_edge(&conn, "thresh-query", "thresh-n2", 10);
    insert_edge(&conn, "thresh-query", "thresh-n3", 10);
    insert_edge(&conn, "thresh-query", "thresh-n4", 10);

    // Weak candidate: only 2 shared neighbors (n1, n2) — below min 3 threshold.
    insert_edge(&conn, "thresh-weak", "thresh-n1", 10);
    insert_edge(&conn, "thresh-weak", "thresh-n2", 10);

    // Strong candidate: 4 shared neighbors (n1..n4) — passes threshold.
    insert_edge(&conn, "thresh-strong", "thresh-n1", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n2", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n3", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n4", 10);

    // Manually insert community assignments — all in community 0.
    let now = chrono::Utc::now().to_rfc3339();
    for name in [
        "thresh-query",
        "thresh-weak",
        "thresh-strong",
        "thresh-n1",
        "thresh-n2",
        "thresh-n3",
        "thresh-n4",
    ] {
        conn.execute(
            "INSERT OR REPLACE INTO concept_communities \
             (concept_name, community_id, modularity, resolution, detected_at) \
             VALUES (?1, 0, 0.5, 1.0, ?2)",
            params![name, now],
        )
        .expect("insert community");
    }
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "thresh-query", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let siblings = parsed["data"]["CommunitySiblings"]
        .as_array()
        .expect("CommunitySiblings");
    let sibling_names: Vec<&str> = siblings.iter().filter_map(|s| s["Name"].as_str()).collect();

    // FR-13.9: Weak candidate (2 shared neighbors) must be filtered out.
    assert!(
        !sibling_names.contains(&"thresh-weak"),
        "FR-13.9: thresh-weak (< 3 shared neighbors) must be filtered out"
    );
    // Control: strong candidate must appear.
    assert!(
        sibling_names.contains(&"thresh-strong"),
        "FR-13.9: thresh-strong (>= 3 shared neighbors) must appear in CommunitySiblings"
    );
}
