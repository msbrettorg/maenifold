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
    assert!(
        direct.iter().any(|relation| {
            matches!(
                relation["Name"].as_str(),
                Some("artificial-intelligence" | "deep-learning")
            )
        }),
        "expected direct relation to AI or Deep Learning: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextWithoutContentTest.
#[test]
fn build_context_without_content_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Machine Learning Concepts for Content Test",
        "This document discusses [[Machine Learning]] and its applications in modern systems.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Machine Learning", 2, 20, Some(false));
    let direct = direct_relations(&context);

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
    let preview_values = direct
        .iter()
        .flat_map(relation_preview_values)
        .collect::<Vec<_>>();

    assert!(
        !direct.is_empty(),
        "should have direct relations from shared concepts: {context}"
    );
    assert!(
        !preview_values.is_empty(),
        "includeContent=true should populate content previews: {context}"
    );
    assert!(
        preview_values.iter().any(|preview| {
            preview.contains("Comprehensive guide")
                || preview.contains("Overview of [[Machine Learning]]")
                || preview.contains("Tutorial on [[Neural Networks]]")
        }),
        "expected previews from contributing files: {context}"
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
