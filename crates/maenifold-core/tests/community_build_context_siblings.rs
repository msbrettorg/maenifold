// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::community::detect_and_persist;
use maenifold_core::json::OutputFormat;
use rusqlite::params;
use serde_json::json;
use support::*;

// T-W1-8: RTM NFR-2 — RAII guard that clears community threshold env vars and restores
// them on drop. Ensures tests run against default thresholds (min_shared=3, min_overlap=0.4)
// regardless of the host environment.
struct CommunityEnvGuard {
    prev_shared: Option<String>,
    prev_overlap: Option<String>,
}

impl CommunityEnvGuard {
    fn enforce_defaults() -> Self {
        let prev_shared = std::env::var("MAENIFOLD_COMMUNITY_MIN_SHARED").ok();
        let prev_overlap = std::env::var("MAENIFOLD_COMMUNITY_MIN_OVERLAP").ok();
        // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
        // so process-wide environment mutations are serialized.
        unsafe {
            std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_SHARED");
            std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_OVERLAP");
        }
        Self {
            prev_shared,
            prev_overlap,
        }
    }
}

impl Drop for CommunityEnvGuard {
    fn drop(&mut self) {
        // SAFETY: all tests in this file hold TEST_MUTEX for their full duration.
        unsafe {
            match &self.prev_shared {
                Some(v) => std::env::set_var("MAENIFOLD_COMMUNITY_MIN_SHARED", v),
                None => std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_SHARED"),
            }
            match &self.prev_overlap {
                Some(v) => std::env::set_var("MAENIFOLD_COMMUNITY_MIN_OVERLAP", v),
                None => std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_OVERLAP"),
            }
        }
    }
}

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
    // With this topology (4 shared neighbors, all weight=10, no private neighbors),
    // NormalizedOverlap = 40/40 = 1.0. Assert > 0.9 to document the expected computation
    // rather than accepting anything above the floor threshold.
    let target_sib = siblings.iter().find(|s| s["Name"] == "sib-target").unwrap();
    let overlap = target_sib["NormalizedOverlap"].as_f64().unwrap_or(0.0);
    assert!(
        overlap > 0.9,
        "FR-13.8: NormalizedOverlap expected ~1.0 (4 shared neighbors, equal weights), got {overlap}"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_SiblingThresholdsFilter (FR-13.9)
//
// Exercises BOTH filter gates in `load_community_siblings`:
//   1. `shared_neighbor_count < min_shared_neighbors` (default 3)
//   2. `normalized_overlap < min_overlap` (default 0.4)
//
// Env vars MAENIFOLD_COMMUNITY_MIN_SHARED and MAENIFOLD_COMMUNITY_MIN_OVERLAP are
// cleared via CommunityEnvGuard to ensure default thresholds apply regardless of host.
#[test]
fn build_context_sibling_thresholds_filter() {
    let _g = TEST_MUTEX.lock();
    let _env_guard = CommunityEnvGuard::enforce_defaults();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    for name in [
        "thresh-query",
        "thresh-weak",
        "thresh-boundary",
        "thresh-strong",
        "thresh-lowoverlap",
        "thresh-n1",
        "thresh-n2",
        "thresh-n3",
        "thresh-n4",
        "thresh-priv1",
    ] {
        insert_concept(&conn, name);
    }

    // Query concept: 4 neighbors (n1..n4) with weight 10 each.
    insert_edge(&conn, "thresh-query", "thresh-n1", 10);
    insert_edge(&conn, "thresh-query", "thresh-n2", 10);
    insert_edge(&conn, "thresh-query", "thresh-n3", 10);
    insert_edge(&conn, "thresh-query", "thresh-n4", 10);

    // Weak candidate: only 2 shared neighbors (n1, n2) — below min_shared=3.
    // Expected: FILTERED by min_shared_neighbors gate.
    insert_edge(&conn, "thresh-weak", "thresh-n1", 10);
    insert_edge(&conn, "thresh-weak", "thresh-n2", 10);

    // Boundary candidate: exactly 3 shared neighbors (n1, n2, n3).
    // shared_count=3 >= min_shared=3 → passes min_shared gate.
    // NormalizedOverlap = 30/30 = 1.0 >= 0.4 → passes overlap gate.
    // Expected: INCLUDED (verifies boundary is inclusive, not off-by-one).
    insert_edge(&conn, "thresh-boundary", "thresh-n1", 10);
    insert_edge(&conn, "thresh-boundary", "thresh-n2", 10);
    insert_edge(&conn, "thresh-boundary", "thresh-n3", 10);

    // Strong candidate: 4 shared neighbors (n1..n4) — well above threshold.
    // NormalizedOverlap = 40/40 = 1.0 >= 0.4 → passes overlap gate.
    // Expected: INCLUDED.
    insert_edge(&conn, "thresh-strong", "thresh-n1", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n2", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n3", 10);
    insert_edge(&conn, "thresh-strong", "thresh-n4", 10);

    // Low-overlap candidate: 3 shared neighbors (passes min_shared=3) but a high-weight
    // private neighbor dilutes overlap below 0.4.
    //   overlap_numerator    = min(10,1) * 3 = 3
    //   sibling_total_degree = 1 + 1 + 1 + 30 = 33
    //   NormalizedOverlap    = 3/33 ≈ 0.091 < 0.4
    // Expected: FILTERED by min_overlap gate (exercises the second filter path).
    insert_edge(&conn, "thresh-lowoverlap", "thresh-n1", 1);
    insert_edge(&conn, "thresh-lowoverlap", "thresh-n2", 1);
    insert_edge(&conn, "thresh-lowoverlap", "thresh-n3", 1);
    insert_edge(&conn, "thresh-lowoverlap", "thresh-priv1", 30);

    // Manually insert community assignments — all in community 0.
    let now = chrono::Utc::now().to_rfc3339();
    for name in [
        "thresh-query",
        "thresh-weak",
        "thresh-boundary",
        "thresh-strong",
        "thresh-lowoverlap",
        "thresh-n1",
        "thresh-n2",
        "thresh-n3",
        "thresh-n4",
        "thresh-priv1",
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

    // FR-13.9 gate 1: Weak candidate (2 shared neighbors < min 3) must be filtered.
    assert!(
        !sibling_names.contains(&"thresh-weak"),
        "FR-13.9: thresh-weak (2 shared neighbors < min 3) must be filtered"
    );

    // FR-13.9 boundary: Candidate with exactly 3 shared neighbors must pass
    // (verifies threshold is `< 3` not `<= 3`).
    assert!(
        sibling_names.contains(&"thresh-boundary"),
        "FR-13.9: thresh-boundary (exactly 3 shared neighbors = min 3) must be included"
    );

    // FR-13.9 gate 2: Low-overlap candidate (3 shared but NormalizedOverlap ≈ 0.091 < 0.4)
    // must be filtered by the overlap gate — exercises the second filter path.
    assert!(
        !sibling_names.contains(&"thresh-lowoverlap"),
        "FR-13.9: thresh-lowoverlap (overlap ~0.091 < min 0.4) must be filtered"
    );

    // Control: strong candidate (4 shared, overlap 1.0) must appear with valid score.
    assert!(
        sibling_names.contains(&"thresh-strong"),
        "FR-13.9: thresh-strong (4 shared neighbors, overlap 1.0) must appear"
    );
    let strong_sib = siblings
        .iter()
        .find(|s| s["Name"] == "thresh-strong")
        .expect("thresh-strong in siblings");
    let strong_overlap = strong_sib["NormalizedOverlap"]
        .as_f64()
        .expect("NormalizedOverlap field");
    assert!(
        strong_overlap > 0.9,
        "FR-13.9: thresh-strong NormalizedOverlap expected ~1.0, got {strong_overlap}"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: BuildContext_IncludesCommunityIdOnRelatedConcepts (FR-13.6)
//
// Positive-case coverage for FR-13.6: when community data exists, BuildContext
// must populate CommunityId on the root concept AND on every DirectRelation.
// The degradation tests cover the inverse (null when no data / orphan).
//
// Merged from community_sync.rs — this test uses detect_and_persist directly
// (not the Sync tool), so it belongs in the build_context suite.
#[test]
fn build_context_includes_community_id_on_related_concepts() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Triangle topology: all three concepts share edges → Louvain places all
    // in the same community deterministically.
    insert_concept(&conn, "ctx-alpha");
    insert_concept(&conn, "ctx-beta");
    insert_concept(&conn, "ctx-gamma");
    insert_edge(&conn, "ctx-alpha", "ctx-beta", 10);
    insert_edge(&conn, "ctx-beta", "ctx-gamma", 10);
    insert_edge(&conn, "ctx-alpha", "ctx-gamma", 10);
    drop(conn);

    let comm_count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    assert!(comm_count > 0, "communities must be detected");

    let conn = open_conn(&env.db_path);
    let persisted = community_row_count(&conn);
    assert!(
        persisted > 0,
        "FR-13.6: communities must be persisted before BuildContext"
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
    let alpha_community_id = data["CommunityId"]
        .as_i64()
        .expect("FR-13.6: query concept ctx-alpha must have non-null CommunityId");

    // Direct relations must have CommunityId populated AND consistent with
    // the triangle topology: all three concepts must share the same community.
    let direct = data["DirectRelations"]
        .as_array()
        .expect("DirectRelations array");
    assert!(!direct.is_empty(), "ctx-alpha must have direct relations");
    for rel in direct {
        let rel_id = rel["CommunityId"].as_i64().unwrap_or_else(|| {
            panic!(
                "FR-13.6: DirectRelation '{}' must have non-null CommunityId",
                rel["Name"]
            )
        });
        assert_eq!(
            rel_id, alpha_community_id,
            "FR-13.6: '{}' must share community with ctx-alpha (triangle topology)",
            rel["Name"]
        );
    }
}
