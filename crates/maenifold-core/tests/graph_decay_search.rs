// T-0.3: RTM NFR-4
#[path = "graph_decay/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, create_test_file_with_age, find_result, isolated_env,
    search_memories_json, sync_graph, title_index,
};

// T-GRAPH-DECAY-001.1 (archived: Sprint 15): RTM FR-7.5
// Merged from three structurally identical per-mode tests into one parameterized test.
// Name corrected: base scores are similar (not equal) since titles differ; the age gap
// dominates any embedding divergence. Each mode retains its own content/query pairing
// because Semantic mode requires vector embeddings that match the query.
#[test]
fn search_memories_decay_raises_newer_file_across_all_modes() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // (mode, content, query, file_prefix, old_age_days, new_age_days)
    let cases: &[(&str, &str, &str, &str, i64, i64)] = &[
        (
            "Hybrid",
            "This document discusses [[machine-learning]] algorithms and [[deep-learning]] neural networks. The primary focus is on supervised [[machine-learning]] techniques for classification tasks.",
            "machine learning deep learning",
            "ml",
            60, 1,
        ),
        (
            "Semantic",
            "Understanding [[vector-embeddings]] for [[semantic-search]] in AI systems.",
            "vector embeddings semantic search",
            "embed",
            45, 2,
        ),
        (
            "FullText",
            "This covers [[authentication]] patterns and [[jwt]] token validation strategies.",
            "authentication jwt token",
            "auth",
            50, 3,
        ),
    ];

    // Create all file pairs up-front so a single sync indexes everything
    for &(_mode, content, _query, prefix, old_age, new_age) in cases {
        create_test_file_with_age(
            &env.cfg,
            &format!("{TEST_FOLDER}/old-{prefix}-doc.md"),
            &format!("Old {prefix} Decay Doc"),
            content,
            old_age,
        );
        create_test_file_with_age(
            &env.cfg,
            &format!("{TEST_FOLDER}/new-{prefix}-doc.md"),
            &format!("New {prefix} Decay Doc"),
            content,
            new_age,
        );
    }

    sync_graph(&env);

    let mut modes_exercised = 0u32;
    for &(mode, _content, query, prefix, old_age, _new_age) in cases {
        let old_title = format!("Old {prefix} Decay Doc");
        let new_title = format!("New {prefix} Decay Doc");

        let search_result = search_memories_json(&env, query, mode);

        assert!(
            search_result["success"].as_bool() == Some(true),
            "[{mode}] search failed: {search_result}"
        );

        // Semantic mode requires an ONNX embedding model; when the model is
        // absent (common in CI/test envs) the search returns 0 results.
        // Gracefully skip rather than fail — mirrors access_boosting_search.rs.
        let results = search_result["data"]["results"]
            .as_array()
            .expect("results array");
        if results.is_empty() {
            eprintln!(
                "NOTE: [{mode}] search returned no results (embedding model likely absent). \
                 Decay assertions skipped for this mode."
            );
            continue;
        }

        // Primary: newer file should rank higher due to decay weighting
        assert!(
            title_index(&search_result, &new_title)
                < title_index(&search_result, &old_title),
            "[{mode}] newer document should rank ahead of older document: {search_result}"
        );

        // Secondary: older file's decayWeight must be < 1.0, proving decay
        // caused the ordering rather than an arbitrary tie-break
        let old_result = find_result(&search_result, &old_title);
        let old_decay = old_result["decayWeight"]
            .as_f64()
            .expect("old file decayWeight");
        assert!(
            old_decay < 1.0,
            "[{mode}] {old_age}-day-old file should carry a decayed weight, got {old_decay}: {search_result}"
        );

        modes_exercised += 1;
    }

    // At minimum Hybrid and FullText must work (they don't need embeddings)
    assert!(
        modes_exercised >= 2,
        "expected at least Hybrid + FullText to return results, but only {modes_exercised} mode(s) were exercised"
    );
}

// T-GRAPH-DECAY-001.1 (archived: Sprint 15): RTM NFR-7.5.2
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

// T-GRAPH-DECAY-001.1 (archived: Sprint 15): RTM NFR-7.5.2
// Companion to the grace-period test above: verifies that a file sufficiently past the
// default 28-day grace window carries a decayed weight < 1.0.
// Note: with the default power-law function (x^-0.5), effective_age=1 (day 29) still
// yields 1.0 because 1^(-0.5) = 1.0. Visible decay starts at effective_age=2 (day 30),
// giving weight ≈ 0.707. We test at day 30 to confirm the transition.
#[test]
fn search_memories_file_just_past_grace_period_has_decayed_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let content = "Testing [[grace-period]] boundary for [[decay-weighting]] at exit.";

    // 30 days = default grace (28) + 2: power-law effective_age=2, weight ≈ 0.707
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/boundary-file.md"),
        "Grace Boundary File",
        content,
        30,
    );

    // Control file well inside the grace period
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/inside-grace-file.md"),
        "Inside Grace File",
        content,
        1,
    );

    sync_graph(&env);

    let search_result =
        search_memories_json(&env, "grace period decay weighting boundary", "Hybrid");

    let boundary = find_result(&search_result, "Grace Boundary File");
    let inside = find_result(&search_result, "Inside Grace File");

    // File at 30 days should have a decayed weight (past grace + power-law effective)
    let boundary_decay = boundary["decayWeight"]
        .as_f64()
        .expect("boundary decayWeight");
    assert!(
        boundary_decay < 1.0,
        "file at 30 days (grace=28, power-law effective_age=2) should have decayWeight < 1.0, got {boundary_decay}"
    );

    // File at 1 day should still be in grace period (weight == 1.0)
    support::assert_approx_eq(
        inside["decayWeight"]
            .as_f64()
            .expect("inside decayWeight"),
        1.0,
        0.000_001,
    );
}
