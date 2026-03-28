// T-12.0: RTM NFR-1
#[allow(clippy::duplicate_mod)]
#[path = "access_boosting/support.rs"]
mod support;

use support::{
    READ_SLEEP_MS, TEST_FOLDER, TEST_MUTEX, assert_search_contains_title, build_context_json,
    create_and_sync_test_file, get_last_accessed, isolated_env, read_memory_output,
    rfc3339_days_ago, search_memories_json, set_last_accessed,
};

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
#[test]
fn build_context_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri1 = create_and_sync_test_file(
        &env,
        "BuildContext Test File 1",
        "Testing [[build-context]] and [[graph-traversal]] concepts.",
        TEST_FOLDER,
    );
    let uri2 = create_and_sync_test_file(
        &env,
        "BuildContext Test File 2",
        "More content about [[build-context]] and [[concept-relationships]].",
        TEST_FOLDER,
    );

    // Verify files exist and are readable in the graph before setting the controlled baseline.
    let read_one = read_memory_output(&env, &uri1);
    let read_two = read_memory_output(&env, &uri2);
    assert!(!read_one.starts_with("ERROR:"), "unexpected: {read_one}");
    assert!(!read_two.starts_with("ERROR:"), "unexpected: {read_two}");

    let baseline_one = rfc3339_days_ago(5);
    let baseline_two = rfc3339_days_ago(4);
    set_last_accessed(&env.cfg, &uri1, &baseline_one);
    set_last_accessed(&env.cfg, &uri2, &baseline_two);

    let initial_last_accessed1 = get_last_accessed(&env.cfg, &uri1);
    let initial_last_accessed2 = get_last_accessed(&env.cfg, &uri2);

    let context_result = build_context_json(&env, "build-context", 2, 20, true);
    let direct_relations = context_result["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array");
    assert!(
        !direct_relations.is_empty(),
        "BuildContext should find related concepts: {context_result}"
    );

    let after_build_context_last_accessed1 = get_last_accessed(&env.cfg, &uri1);
    let after_build_context_last_accessed2 = get_last_accessed(&env.cfg, &uri2);

    assert_eq!(
        after_build_context_last_accessed1, initial_last_accessed1,
        "BuildContext should not update last_accessed for the first referenced file"
    );
    assert_eq!(
        after_build_context_last_accessed2, initial_last_accessed2,
        "BuildContext should not update last_accessed for the second referenced file"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3 — multi-hop traversal must not update last_accessed
#[test]
fn build_context_deep_traversal_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Deep Traversal Test",
        "Testing [[deep-traversal]] with [[graph-depth]] concepts.",
        TEST_FOLDER,
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial = get_last_accessed(&env.cfg, &uri);

    // Depth=3 exercises the full hop-expansion loop in expand_related_concepts
    build_context_json(&env, "deep-traversal", 3, 50, false);

    let after = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after, initial,
        "BuildContext at depth=3 (multi-hop expansion) must not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1, NFR-7.6.2, NFR-7.6.3
#[test]
fn only_read_memory_updates_last_accessed_not_search_or_build_context() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Combined Behavior Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[combined-access]] patterns for [[decay-weighting]] and [[boosting]].",
        TEST_FOLDER,
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);

    let first_read_result = read_memory_output(&env, &uri);
    assert!(
        !first_read_result.starts_with("ERROR:"),
        "first read should succeed: {first_read_result}"
    );
    let after_first_read = get_last_accessed(&env.cfg, &uri).expect("last_accessed after read");

    let search_result =
        search_memories_json(&env, "combined-access decay", "Hybrid", Some(TEST_FOLDER));
    assert_search_contains_title(&search_result, title);
    let after_search = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search,
        Some(after_first_read),
        "SearchMemories should not update last_accessed"
    );

    let context_result = build_context_json(&env, "combined-access", 2, 20, true);
    assert_eq!(
        context_result["data"]["ConceptName"].as_str(),
        Some("combined-access"),
        "unexpected BuildContext output: {context_result}"
    );
    let after_build_context = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_build_context,
        Some(after_first_read),
        "BuildContext should not update last_accessed"
    );

    std::thread::sleep(std::time::Duration::from_millis(READ_SLEEP_MS));

    let second_read_result = read_memory_output(&env, &uri);
    assert!(
        !second_read_result.starts_with("ERROR:"),
        "second read should succeed: {second_read_result}"
    );
    let after_second_read =
        get_last_accessed(&env.cfg, &uri).expect("last_accessed after second read");
    assert!(
        after_second_read > after_first_read,
        "a subsequent ReadMemory should update last_accessed"
    );
}
