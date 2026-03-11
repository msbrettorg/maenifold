// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use chrono::Utc;
use maenifold_core::community::compute_edge_decay_weight;
use serde_json::json;
use std::fs;
use support::{
    TestEnv, acquire_test_lock, assert_approx_eq, direct_relation, insert_concept,
    insert_edge_raw_source_files, insert_edge_with_proximity, invoke_build_context_json,
    memory_uri_from_relative, open_db,
};

// T-4.3: RTM FR-23.4
#[test]
fn compute_edge_decay_weight_returns_proximity_for_fresh_file() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();

    let relative_path = "projects/prox-decay-test.md";
    let file_path = env.cfg.memory_path.join(relative_path);
    let created_utc = Utc::now().to_rfc3339();
    fs::create_dir_all(file_path.parent().expect("parent")).expect("create parent dirs");
    fs::write(
        &file_path,
        format!(
            "---\ntitle: Proximity Decay Test\ncreated: {created_utc}\n---\n\nContent with [[test-concept]].\n"
        ),
    )
    .expect("write proximity decay file");

    let uri = memory_uri_from_relative(relative_path);
    let effective_weight = compute_edge_decay_weight(&json!({ uri: 0.5 }).to_string())
        .expect("compute edge decay weight");

    assert!(effective_weight > 0.0, "weight should remain positive");
    assert!(
        effective_weight <= 0.5 + 1e-6,
        "weight should not exceed proximity"
    );
    assert!(
        effective_weight > 0.4,
        "fresh file weight should remain close to proximity"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn compute_edge_decay_weight_rejects_corrupt_source_files_json() {
    let _guard = acquire_test_lock();

    let result = compute_edge_decay_weight("this is not valid json {{{{");
    assert!(result.is_err(), "corrupt source_files must fail");
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_uses_proximity_weight_in_decay_weight() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "prox-query");
    insert_concept(&conn, "prox-strong");
    insert_concept(&conn, "prox-weak");

    let uri = "memory://test/prox-buildcontext";
    insert_edge_with_proximity(&conn, "prox-query", "prox-strong", uri, 1.0);
    insert_edge_with_proximity(&conn, "prox-query", "prox-weak", uri, 0.5);
    drop(conn);

    let payload = invoke_build_context_json("prox-query");
    let strong = direct_relation(&payload, "prox-strong");
    let weak = direct_relation(&payload, "prox-weak");

    assert!(
        strong["DecayWeight"].as_f64().expect("strong decay")
            > weak["DecayWeight"].as_f64().expect("weak decay"),
        "strong proximity relation must have higher DecayWeight: {payload}"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_ranks_stronger_proximity_above_weaker_proximity() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "rank-query");
    insert_concept(&conn, "rank-strong");
    insert_concept(&conn, "rank-weak");

    let uri = "memory://test/prox-ranking";
    insert_edge_with_proximity(&conn, "rank-query", "rank-strong", uri, 1.0);
    insert_edge_with_proximity(&conn, "rank-query", "rank-weak", uri, 0.5);
    drop(conn);

    let payload = invoke_build_context_json("rank-query");
    let strong = direct_relation(&payload, "rank-strong");
    let weak = direct_relation(&payload, "rank-weak");

    assert!(
        strong["WeightedScore"].as_f64().expect("strong score")
            > weak["WeightedScore"].as_f64().expect("weak score"),
        "strong proximity relation must have higher WeightedScore: {payload}"
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
