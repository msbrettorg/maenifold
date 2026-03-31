// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use maenifold_core::community::{compute_nmi, detect_and_persist};
use serde_json::json;
use std::collections::BTreeMap;
use support::{
    EnvVarGuard, TEST_MUTEX, TestEnv, community_id, insert_concept,
    insert_edge_raw_source_files, insert_edge_with_proximity, open_db,
};

// ---------- helpers for parameterized NMI cases ----------

/// A single NMI test vector: two clusterings and the expected NMI value.
struct NmiCase {
    label: &'static str,
    left: BTreeMap<String, usize>,
    right: BTreeMap<String, usize>,
    expected: NmiExpectation,
}

enum NmiExpectation {
    /// NMI must be approximately equal to this value (within 1e-9).
    Approx(f64),
    /// NMI must fall strictly within (lo, hi) exclusive.
    Range { lo: f64, hi: f64 },
}

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_clusters_hubs_together_when_proximity_sum_is_strongest() {
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    let strong_edge = json!({ "memory://test/prox-h-heavy.md": 100.0 }).to_string();
    let satellite_edge = json!({ "memory://test/prox-h-sat.md": 1.0 }).to_string();

    insert_concept(&conn, "prox-h-a");
    insert_concept(&conn, "prox-h-b");
    insert_edge_raw_source_files(&conn, "prox-h-a", "prox-h-b", 1.0, &strong_edge);

    for name in ["prox-h-s1", "prox-h-s2", "prox-h-s3"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-a", name, 1.0, &satellite_edge);
    }
    for name in ["prox-h-s4", "prox-h-s5", "prox-h-s6"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-b", name, 1.0, &satellite_edge);
    }
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 0.5, None).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_eq!(
        community_id(&conn, "prox-h-a"),
        community_id(&conn, "prox-h-b")
    );
}

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_treats_empty_source_files_as_isolated_edges() {
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    // R-5: Anchor pair removed — it was dead weight never asserted on.
    // Two concepts with `{}` source_files produce a 0-weight edge that is dropped
    // from the adjacency list, leaving both nodes as singletons.
    insert_concept(&conn, "prox-isolated-x");
    insert_concept(&conn, "prox-isolated-y");
    insert_edge_raw_source_files(&conn, "prox-isolated-x", "prox-isolated-y", 5.0, "{}");
    drop(conn);

    // R-2: seed for deterministic Louvain ordering
    detect_and_persist(&env.cfg.database_path, 1.0, Some(0)).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_ne!(
        community_id(&conn, "prox-isolated-x"),
        community_id(&conn, "prox-isolated-y"),
        "empty source_files must produce an isolated pair"
    );
}

// T-4.3: RTM FR-23.7
// R-3: No acquire_test_lock — compute_nmi is a pure function with no side effects.
// R-4: Merged former Tests 3 & 4 into a single parameterized suite.
// R-1: Orthogonal partition now asserts ≈ 0.0 instead of the former weak `< 1.0`.
#[test]
fn compute_nmi_produces_correct_values_for_known_partitions() {
    let _guard = TEST_MUTEX.lock();
    let cases = vec![
        // Case 1: Identical partition → NMI = 1.0 (mathematical identity)
        NmiCase {
            label: "identical partition",
            left: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 0),
                ("gamma".into(), 1),
                ("delta".into(), 1),
            ]),
            right: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 0),
                ("gamma".into(), 1),
                ("delta".into(), 1),
            ]),
            expected: NmiExpectation::Approx(1.0),
        },
        // Case 2: Perfectly orthogonal 2×2 partition → MI = 0, NMI = 0.0
        // Every (left_cluster, right_cluster) cell in the 2×2 contingency table
        // contains exactly one element; mutual information is zero.
        NmiCase {
            label: "orthogonal 2x2 partition",
            left: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 0),
                ("gamma".into(), 1),
                ("delta".into(), 1),
            ]),
            right: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 1),
                ("gamma".into(), 0),
                ("delta".into(), 1),
            ]),
            expected: NmiExpectation::Approx(0.0),
        },
        // Case 3: Partial overlap — one concept moved between clusters
        // 3 of 4 assignments agree → 0 < NMI < 1
        NmiCase {
            label: "partial overlap (one concept moved)",
            left: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 0),
                ("gamma".into(), 1),
                ("delta".into(), 1),
            ]),
            right: BTreeMap::from([
                ("alpha".into(), 0_usize),
                ("beta".into(), 0),
                ("gamma".into(), 0),
                ("delta".into(), 1),
            ]),
            expected: NmiExpectation::Range { lo: 0.0, hi: 1.0 },
        },
        // Case 4: Disjoint concept sets — implementation returns 1.0 for empty intersection
        NmiCase {
            label: "disjoint concept sets",
            left: BTreeMap::from([("a".into(), 0_usize), ("b".into(), 1)]),
            right: BTreeMap::from([("c".into(), 0_usize), ("d".into(), 1)]),
            expected: NmiExpectation::Approx(1.0),
        },
    ];

    for case in &cases {
        let nmi = compute_nmi(&case.left, &case.right);
        // Range guard: NMI must always be normalized
        assert!(
            (0.0..=1.0).contains(&nmi),
            "[{label}] NMI must remain normalized, got {nmi}",
            label = case.label,
        );
        match &case.expected {
            NmiExpectation::Approx(expected) => {
                assert!(
                    (nmi - expected).abs() <= 1e-9,
                    "[{label}] expected NMI ≈ {expected:.10}, got {nmi:.10}",
                    label = case.label,
                );
            }
            NmiExpectation::Range { lo, hi } => {
                assert!(
                    nmi > *lo && nmi < *hi,
                    "[{label}] expected NMI in ({lo}, {hi}), got {nmi}",
                    label = case.label,
                );
            }
        }
    }
}

// T-14.6: RTM FR-15
// R-2: Seeded both detect_and_persist calls for deterministic Louvain ordering.
// R-6: Strengthened assertion — require both "NMI" keyword and a numeric value near 0.0.
#[test]
#[cfg(unix)]
fn detect_and_persist_logs_warning_when_community_drift_exceeds_threshold() {
    let _threshold = EnvVarGuard::set("MAENIFOLD_COMMUNITY_NMI_THRESHOLD", "0.95");
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    for name in ["drift-a0", "drift-a1", "drift-a2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a1",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );

    for name in ["drift-b0", "drift-b1", "drift-b2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b1",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b1",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 1.0, Some(42)).expect("first detect");

    let conn = open_db(&env.cfg);
    conn.execute("DELETE FROM concept_graph", [])
        .expect("clear graph");
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-b0",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-b1",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a2",
        "drift-b2",
        "memory://test/drift-cross.md",
        1.0,
    );
    drop(conn);

    let (_, stderr_output) = support::capture_stderr(|| {
        detect_and_persist(&env.cfg.database_path, 1.0, Some(42)).expect("second detect");
    });

    // R-6: Pin both the mechanism (NMI keyword) AND the expected metric range (≈ 0.0).
    assert!(
        stderr_output.contains("NMI") && stderr_output.contains("0.00"),
        "expected drift warning with NMI near 0.0 in stderr, got: {stderr_output}"
    );
}
