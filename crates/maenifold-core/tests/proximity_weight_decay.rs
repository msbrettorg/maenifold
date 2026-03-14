// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use maenifold_core::community::compute_edge_decay_weight;
use support::{
    TestEnv, acquire_test_lock, assert_approx_eq, direct_relation, insert_concept,
    insert_edge_raw_source_files, insert_edge_with_proximity, insert_file_content,
    invoke_build_context_json, open_db,
};

// T-4.3: RTM FR-23.4 — Arithmetic mean of two proximity weights.
// Exercises the sum/len path that is trivially identity for N=1.
#[test]
fn compute_edge_decay_weight_averages_multiple_proximity_weights() {
    let result =
        compute_edge_decay_weight(r#"{"memory://a": 1.0, "memory://b": 0.5}"#).expect("valid json");
    assert_approx_eq(result, 0.75); // (1.0 + 0.5) / 2
}

// T-4.3: RTM FR-23.4 — Single-entry passthrough returns the exact weight.
#[test]
fn compute_edge_decay_weight_single_entry_returns_that_weight() {
    let result = compute_edge_decay_weight(r#"{"memory://x": 0.5}"#).expect("valid json");
    assert_approx_eq(result, 0.5);
}

// T-4.3: RTM FR-23.4 — Error path: corrupt JSON propagates Err.
#[test]
fn compute_edge_decay_weight_rejects_corrupt_source_files_json() {
    let result = compute_edge_decay_weight("this is not valid json {{{{");
    assert!(result.is_err(), "corrupt source_files must fail");
}

// T-4.3: RTM FR-23.4 — Valid JSON that is neither object nor array returns Ok(0.0).
// parse_source_file_weights returns an empty BTreeMap for these types.
#[test]
fn compute_edge_decay_weight_returns_zero_for_json_non_object() {
    assert_approx_eq(compute_edge_decay_weight("null").expect("null"), 0.0);
    assert_approx_eq(compute_edge_decay_weight("42").expect("number"), 0.0);
    assert_approx_eq(compute_edge_decay_weight("true").expect("bool"), 0.0);
}

// T-4.3: RTM FR-23.4 — Merged from former T3+T4: both DecayWeight and WeightedScore
// reflect proximity ordering. When co_occurrence_count == 1 and no file_content row
// exists (neutral fallback 1.0), WeightedScore == DecayWeight numerically.
#[test]
fn build_context_decayweight_and_weightedscore_reflect_proximity_ordering() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "prox-merged-query");
    insert_concept(&conn, "prox-merged-strong");
    insert_concept(&conn, "prox-merged-weak");

    let uri = "memory://test/prox-merged";
    insert_edge_with_proximity(&conn, "prox-merged-query", "prox-merged-strong", uri, 1.0);
    insert_edge_with_proximity(&conn, "prox-merged-query", "prox-merged-weak", uri, 0.5);
    drop(conn);

    let payload = invoke_build_context_json("prox-merged-query");
    let strong = direct_relation(&payload, "prox-merged-strong");
    let weak = direct_relation(&payload, "prox-merged-weak");

    assert!(
        strong["DecayWeight"].as_f64().expect("strong decay")
            > weak["DecayWeight"].as_f64().expect("weak decay"),
        "strong proximity must have higher DecayWeight: {payload}"
    );
    // co_occurrence_count = 1, so WeightedScore == DecayWeight; if they ever diverge, update.
    assert!(
        strong["WeightedScore"].as_f64().expect("strong score")
            > weak["WeightedScore"].as_f64().expect("weak score"),
        "strong proximity must have higher WeightedScore: {payload}"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_treats_empty_source_files_as_zero_decay_weight() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "empty-query");
    insert_concept(&conn, "empty-peer");
    insert_edge_raw_source_files(&conn, "empty-query", "empty-peer", 1.0, "{}");
    drop(conn);

    let payload = invoke_build_context_json("empty-query");
    let peer = direct_relation(&payload, "empty-peer");

    assert_approx_eq(peer["DecayWeight"].as_f64().expect("decay weight"), 0.0);
}

// T-4.3: RTM FR-23.4 — Temporal decay fires when file_content has old timestamps.
// Without this test, the time-decay multiply in graph/support.rs relation_weighted_score
// is never exercised: all other tests use URIs absent from file_content, so
// load_file_decay_weight returns the neutral fallback 1.0.
#[test]
fn build_context_decayweight_reduced_for_old_file() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    // Insert a file_content row with timestamps well past the grace period (28 days default).
    // Power-law decay at 365+ days: (365-28)^(-0.5) ≈ 0.054, far below 1.0.
    let old_date = "2024-01-01T00:00:00Z";
    let uri = "memory://test/old-decay-file";
    insert_file_content(&conn, uri, old_date, Some(old_date));

    insert_concept(&conn, "temporal-query");
    insert_concept(&conn, "temporal-peer");
    insert_edge_with_proximity(&conn, "temporal-query", "temporal-peer", uri, 1.0);
    drop(conn);

    let payload = invoke_build_context_json("temporal-query");
    let peer = direct_relation(&payload, "temporal-peer");

    let decay_weight = peer["DecayWeight"].as_f64().expect("decay weight");
    assert!(
        decay_weight < 1.0,
        "DecayWeight should be reduced by temporal decay for old file, got {decay_weight}: {payload}"
    );
    assert!(
        decay_weight > 0.0,
        "DecayWeight should still be positive, got {decay_weight}: {payload}"
    );
}
