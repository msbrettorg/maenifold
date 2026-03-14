// T-12.0: RTM NFR-1, NFR-2
#[path = "access_boosting/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, assert_search_contains_title, create_and_sync_test_file,
    get_last_accessed, isolated_env, read_memory_output, rfc3339_days_ago, search_memories_json,
    set_last_accessed,
};

/// Shared assertion: searching in `mode` must not mutate `last_accessed`.
///
/// Creates a test file, reads it once to flush initialization side-effects,
/// sets a known baseline timestamp, performs a search, and verifies the
/// timestamp is unchanged. Returns `true` if the search produced results
/// confirming the file was found; `false` if the result set was empty.
///
/// Caller MUST hold `TEST_MUTEX` before invoking — `Config::override_root()`
/// mutates process-global state.
fn assert_search_does_not_mutate_last_accessed(
    mode: &str,
    title: &str,
    content: &str,
    query: &str,
) -> bool {
    let env = isolated_env();
    let uri = create_and_sync_test_file(&env, title, content, TEST_FOLDER);

    // Warm the file: ReadMemory sets last_accessed; we overwrite it immediately
    // with a known baseline so the subsequent search comparison is clean.
    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(&env, query, mode, Some(TEST_FOLDER));

    let has_results = search_result["data"]["results"]
        .as_array()
        .map_or(false, |r| !r.is_empty());

    if has_results {
        assert_search_contains_title(&search_result, title);
    }

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "{mode} SearchMemories should not update last_accessed"
    );

    has_results
}

// NFR-1, NFR-2: Hybrid search must not boost last_accessed.
#[test]
fn search_memories_hybrid_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let found = assert_search_does_not_mutate_last_accessed(
        "Hybrid",
        "Search Results Test",
        "Testing [[search-behavior]] for [[access-boosting]]. This contains machine learning keywords.",
        "search-behavior access-boosting",
    );
    assert!(found, "Hybrid search should always return results");
}

// NFR-1, NFR-2: Semantic search must not boost last_accessed.
// When vec0 extension is absent, semantic search returns no results and the invariant
// is validated against an empty result set only. The test still passes but logs a skip
// notice rather than silently claiming full coverage.
#[test]
fn search_memories_semantic_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let found = assert_search_does_not_mutate_last_accessed(
        "Semantic",
        "Semantic Search Test",
        "Testing [[semantic-search]] and [[vector-embeddings]] behavior.",
        "semantic search vector",
    );
    if !found {
        eprintln!(
            "NOTE: Semantic search returned no results (vec0 extension likely absent). \
             The last_accessed invariant was still validated, but the semantic code path \
             was not fully exercised."
        );
    }
}

// NFR-1, NFR-2: FullText search must not boost last_accessed.
#[test]
fn search_memories_fulltext_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let found = assert_search_does_not_mutate_last_accessed(
        "FullText",
        "FullText Search Test",
        "Testing [[fulltext-search]] and [[keyword-matching]] behavior.",
        "fulltext keyword matching",
    );
    assert!(found, "FullText search should always return results");
}
