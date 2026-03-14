// T-12.7: RTM NFR-1
#[path = "concept_graph/support.rs"]
mod support;

use support::{
    TEST_MUTEX, build_context_json, direct_relations, isolated_env, relation_preview_values,
    sync_graph, write_memory_file,
};

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs ConceptGraphSmokeTest.
#[test]
fn concept_graph_smoke_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "AI Concepts Test",
        "This document explores [[Artificial Intelligence]] and its relationship to [[Machine Learning]] and [[Deep Learning]].\n\n[[Deep Learning]] is a subset of [[Machine Learning]], which itself is a subset of [[Artificial Intelligence]].",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Machine Learning", 2, 20, Some(false));
    let direct = direct_relations(&context);

    assert!(
        !direct.is_empty(),
        "expected BuildContext to return related concepts: {context}"
    );

    // T-1.6: RTM NFR-1 — assert both neighbours individually, not via permissive OR.
    let names: Vec<&str> = direct.iter().filter_map(|r| r["Name"].as_str()).collect();
    assert!(
        names.contains(&"artificial-intelligence"),
        "expected AI as direct relation of Machine Learning: {context}"
    );
    assert!(
        names.contains(&"deep-learning"),
        "expected Deep Learning as direct relation of Machine Learning: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextWithoutContentTest.
#[test]
fn build_context_without_content_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // T-1.6: RTM NFR-1 — two WikiLinks required for a graph edge so DirectRelations
    // is non-empty and the `all()` assertion below is non-vacuous.
    write_memory_file(
        &env,
        "Machine Learning Concepts for Content Test",
        "[[Machine Learning]] enables [[Deep Learning]] applications in modern systems.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Machine Learning", 2, 20, Some(false));
    let direct = direct_relations(&context);

    // Pre-condition: relations must exist so `all()` is non-vacuous.
    assert!(
        !direct.is_empty(),
        "expected at least one relation to make content-flag assertion non-vacuous: {context}"
    );
    assert!(
        direct
            .iter()
            .all(|relation| relation_preview_values(relation).is_empty()),
        "includeContent=false should not populate previews: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextWithContentTest.
#[test]
fn build_context_with_content_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Deep Learning Guide",
        "Comprehensive guide to [[Deep Learning]] using [[Neural Networks]]. [[Deep Learning]] is part of [[Machine Learning]].",
    );
    write_memory_file(
        &env,
        "Machine Learning Overview",
        "Overview of [[Machine Learning]] and [[Deep Learning]] in [[AI]]. [[Machine Learning]] uses [[Neural Networks]].",
    );
    write_memory_file(
        &env,
        "Neural Networks Tutorial",
        "Tutorial on [[Neural Networks]] for [[Deep Learning]]. [[Neural Networks]] power modern [[AI]].",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Deep Learning", 2, 20, Some(true));
    let direct = direct_relations(&context);

    assert!(
        !direct.is_empty(),
        "should have direct relations from shared concepts: {context}"
    );

    // T-1.6: RTM NFR-1 — collect all preview values across all relations and verify
    // multi-file aggregation by counting distinct contributing files rather than
    // using a permissive OR that passes even when most files are missing.
    let total_preview_count: usize = direct
        .iter()
        .map(|r| r["ContentPreview"].as_object().map_or(0, |map| map.len()))
        .sum();

    assert!(
        total_preview_count > 0,
        "includeContent=true should populate content previews: {context}"
    );
    assert!(
        total_preview_count > 1,
        "expected previews from multiple contributing files, got {total_preview_count}: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextIncludeContentDefaultTest.
#[test]
fn build_context_include_content_default_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "AI Overview for Default Test",
        "Overview of [[Artificial Intelligence]] and [[Modern Systems]] in production environments.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Artificial Intelligence", 2, 20, None);
    let direct = direct_relations(&context);

    assert!(
        !direct.is_empty(),
        "default-path test should include at least one relation: {context}"
    );
    assert!(
        direct
            .iter()
            .all(|relation| relation_preview_values(relation).is_empty()),
        "default includeContent should remain false: {context}"
    );
}
