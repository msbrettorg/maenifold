// T-0.3: RTM NFR-4
#[path = "graph_decay/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, assert_approx_eq, create_test_file_with_age, decay_weight_for_uri,
    get_last_accessed, isolated_env, read_memory_markdown, search_memories_json, sync_graph,
};

// T-GRAPH-DECAY-002.1: RTM NFR-7.6.1
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

    let weight_before_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_before_access < 0.8,
        "60-day-old file should have significant decay before access boosting: {weight_before_access}"
    );

    let read_result = read_memory_markdown(&env, &uri);
    assert!(!read_result.starts_with("ERROR:"), "{read_result}");

    let weight_after_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_after_access > weight_before_access,
        "weight should increase after ReadMemory: before={weight_before_access}, after={weight_after_access}"
    );
    assert!(
        weight_after_access >= 0.95,
        "recently read file should have near-full weight: {weight_after_access}"
    );
}

// T-GRAPH-DECAY-002.1: RTM NFR-7.6.1
#[test]
fn get_decay_weight_after_search_memories_weight_is_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/search-no-boost.md"),
        "Search No Boost",
        "This document discusses [[search-no-boost]] and [[passive-access]] patterns.",
        60,
    );

    sync_graph(&env);

    let initial_last_accessed = get_last_accessed(&env.cfg, &uri).expect("initial last_accessed");
    let weight_before = decay_weight_for_uri(&env.cfg, &uri);

    let _search_result = search_memories_json(&env, "search no boost passive access", "Hybrid");

    let after_last_accessed =
        get_last_accessed(&env.cfg, &uri).expect("last_accessed after search");
    let weight_after = decay_weight_for_uri(&env.cfg, &uri);

    assert_eq!(
        after_last_accessed, initial_last_accessed,
        "SearchMemories should not update last_accessed"
    );
    assert_approx_eq(weight_after, weight_before, 0.000_001);
}
