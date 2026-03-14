// T-0.3: RTM NFR-4
#[path = "graph_decay/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, assert_approx_eq, create_test_file_with_age, decay_weight_for_uri,
    isolated_env, read_memory_markdown, sync_graph,
};

// T-GRAPH-DECAY-002.1: RTM-201
// End-to-end weight-layer integration: proves that a live ReadMemory call
// produces a measurable change in the output of `decay_weight_for_uri`.
// Bridges the DB write (last_accessed update) and the weight calculation
// layers — access_boosting_reads.rs covers the timestamp layer; this test
// covers the *weight* layer.
//
// Precondition: On first sync of a file whose frontmatter `created` is
// 60 days in the past, the file_content.last_accessed column is seeded
// from that frontmatter date (via COALESCE in sync/file_index.rs INSERT).
// This gives the file a decayed weight < 0.8 before any ReadMemory call.
#[test]
fn get_decay_weight_after_read_memory_returns_higher_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/access-boost-test.md"),
        "Access Boost Test",
        "This document discusses [[access-boosting]] and [[decay-reset]] behavior.",
        60,
    );

    sync_graph(&env);

    // Precondition: 60-day-old file should have significant decay.
    // last_accessed was seeded from frontmatter `created` (60 days ago).
    let weight_before_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_before_access < 0.8,
        "60-day-old file should have significant decay before access boosting: {weight_before_access}"
    );

    let read_result = read_memory_markdown(&env, &uri);
    assert!(!read_result.starts_with("ERROR:"), "{read_result}");

    // ReadMemory sets last_accessed = Utc::now(), which means age_days ≈ 0.
    // With age within the grace period, weight should be exactly 1.0.
    let weight_after_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_after_access > weight_before_access,
        "weight should increase after ReadMemory: before={weight_before_access}, after={weight_after_access}"
    );
    assert_approx_eq(
        weight_after_access,
        1.0,
        0.01,
    );
}
