// T-12.0: RTM NFR-1
#[path = "access_boosting/support.rs"]
mod support;

use maenifold_core::vector;
use support::{
    TEST_FOLDER, TEST_MUTEX, assert_search_contains_title, create_and_sync_test_file,
    get_last_accessed, isolated_env, read_memory_output, rfc3339_days_ago, search_memories_json,
    set_last_accessed,
};

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
fn search_memories_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Search Results Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[search-behavior]] for [[access-boosting]]. This contains machine learning keywords.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "search-behavior access-boosting",
        "Hybrid",
        Some(TEST_FOLDER),
    );
    assert_search_contains_title(&search_result, title);

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "SearchMemories should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
fn search_memories_semantic_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Semantic Search Test";
    // T-8.4: RTM FR-33.1 — ensure the semantic-path regression test exercises the real ONNX runtime.
    let probe = vector::generate_embedding("semantic search vector").expect("probe embedding");
    assert_eq!(probe.len(), vector::EMBEDDING_DIMENSIONS);
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[semantic-search]] and [[vector-embeddings]] behavior.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "semantic search vector",
        "Semantic",
        Some(TEST_FOLDER),
    );
    // T-13.1: Semantic mode now correctly uses vector-only search. If vec0 extension
    // is not in the test environment's assets_path, results will be empty. The core
    // assertion (last_accessed unchanged) still holds regardless.
    let results = search_result["data"]["results"].as_array();
    if results.map_or(false, |r| !r.is_empty()) {
        assert_search_contains_title(&search_result, title);
    }

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "Semantic SearchMemories should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
fn search_memories_fulltext_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "FullText Search Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[fulltext-search]] and [[keyword-matching]] behavior.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "fulltext keyword matching",
        "FullText",
        Some(TEST_FOLDER),
    );
    assert_search_contains_title(&search_result, title);

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "FullText SearchMemories should not update last_accessed"
    );
}
