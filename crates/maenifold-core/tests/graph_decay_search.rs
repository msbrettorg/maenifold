// T-0.3: RTM NFR-4
#[path = "graph_decay/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, create_test_file_with_age, find_result, isolated_env,
    search_memories_json, sync_graph, title_index,
};

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_newer_content_ranks_higher_when_scores_are_equal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content = "This document discusses [[machine-learning]] algorithms and [[deep-learning]] neural networks. The primary focus is on supervised [[machine-learning]] techniques for classification tasks.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-ml-doc.md"),
        "Old Machine Learning Doc",
        identical_content,
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-ml-doc.md"),
        "New Machine Learning Doc",
        identical_content,
        1,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "machine learning deep learning", "Hybrid");

    assert!(
        search_result["success"].as_bool() == Some(true),
        "{search_result}"
    );
    assert!(
        title_index(&search_result, "New Machine Learning Doc")
            < title_index(&search_result, "Old Machine Learning Doc"),
        "newer document should rank ahead of older document: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_semantic_mode_applies_decay_weighting() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content =
        "Understanding [[vector-embeddings]] for [[semantic-search]] in AI systems.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-embeddings.md"),
        "Old Embeddings Doc",
        identical_content,
        45,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-embeddings.md"),
        "New Embeddings Doc",
        identical_content,
        2,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "vector embeddings semantic search", "Semantic");

    assert!(
        title_index(&search_result, "New Embeddings Doc")
            < title_index(&search_result, "Old Embeddings Doc"),
        "newer document should rank ahead in semantic mode: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_fulltext_mode_applies_decay_weighting() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content =
        "This covers [[authentication]] patterns and [[jwt]] token validation strategies.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth.md"),
        "Old Authentication Doc",
        identical_content,
        50,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-auth.md"),
        "New Authentication Doc",
        identical_content,
        3,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "authentication jwt token", "FullText");

    assert!(
        title_index(&search_result, "New Authentication Doc")
            < title_index(&search_result, "Old Authentication Doc"),
        "newer document should rank ahead in full-text mode: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM NFR-7.5.2
#[test]
fn search_memories_files_within_grace_period_have_equal_ranking_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content = "Discussing [[grace-period]] behavior for [[decay-weighting]] tests.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/day1-file.md"),
        "Day 1 File",
        identical_content,
        1,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/day10-file.md"),
        "Day 10 File",
        identical_content,
        10,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "grace period decay weighting", "Hybrid");
    let day1 = find_result(&search_result, "Day 1 File");
    let day10 = find_result(&search_result, "Day 10 File");

    support::assert_approx_eq(
        day1["decayWeight"].as_f64().expect("day1 decay weight"),
        1.0,
        0.000_001,
    );
    support::assert_approx_eq(
        day10["decayWeight"].as_f64().expect("day10 decay weight"),
        1.0,
        0.000_001,
    );
}
