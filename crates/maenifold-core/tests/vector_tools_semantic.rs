#[path = "vector_tools/support.rs"]
mod vector_tools_support;

use maenifold_core::json::OutputFormat;
use maenifold_core::repair;
use maenifold_core::tools::ToolContext;
use serde_json::json;
use std::thread;
use vector_tools_support::*;

// T-12.6: RTM NFR-1 — semantic validation regressions stay covered after splitting vector_tools_tests.rs.
// T-12.7: RTM NFR-1 — acceptance hygiene trims mock-dependent semantic vector tests that overclaim behavior.
// T-0.1: RTM NFR-3 — Ports the C# VectorToolsTests file into the Rust oracle.
// T-14.9: RTM FR-6 / RTM-042 — below-threshold or zero-vector semantic validation
// must return a successful BLOCKED warning payload rather than Err(..).
#[test]
fn null_and_empty_embeddings_produce_blocked_warning() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/empty-null.md"),
        "# Test\nUsing [[tools]] with empty vec rows.\n",
    );

    insert_null_embedding(&env, "tool");
    insert_text_embedding(&env, "tools", "");

    let output = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.10
        }),
    )
    .expect("degenerate embeddings should return BLOCKED warning text instead of Err");

    assert!(
        output.contains("BLOCKED:"),
        "expected semantic-validation BLOCKED warning output, got: {output}"
    );
    assert!(
        output.contains("tools: similarity 0.00 (threshold: 0.10)"),
        "expected actual vs required similarity details, got: {output}"
    );
    assert!(
        !output.contains("Scanning"),
        "blocked validation should short-circuit before scanning files: {output}"
    );
}

// T-0.1: RTM NFR-3 — Adapted from C# ThreadSafeInitializationWorks. In Rust this
// validates concurrent repair_concepts reads over shared SQLite vec_concepts rows
// produce identical output on every thread — not initialization lifecycle.
// T-14.9: RTM FR-6 / RTM-043 — above-threshold similarity should still succeed.
#[test]
fn concurrent_repair_concepts_reads_produce_consistent_output() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/concurrent.md"),
        "# Test\nUsing [[tools]] in concurrent dry-run repairs.\n",
    );

    let embedding = deterministic_embedding(0.75);
    insert_blob_embedding(&env, "tool", &embedding);
    insert_blob_embedding(&env, "tools", &embedding);

    let handles = (0..10)
        .map(|_| {
            let cfg = env.cfg.clone();
            thread::spawn(move || {
                let context = ToolContext::new(cfg, OutputFormat::Markdown);
                repair::repair_concepts(
                    &context,
                    &json!({
                        "conceptsToReplace": "tools",
                        "canonicalConcept": "tool",
                        "dryRun": true,
                        "minSemanticSimilarity": 0.99
                    }),
                )
                .expect("threaded repair concepts")
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let output = handle.join().expect("thread join");
        assert!(
            output.contains("All variants pass semantic similarity check")
                && output.contains("Total replacements: 1"),
            "each thread should observe the same successful validation: {output}"
        );
    }
}

// T-14.9: RTM FR-6 / RTM-043 — above-threshold semantic validation remains an Ok
// success payload after the blocked-path transport semantics change.
#[test]
fn above_threshold_similarity_returns_success_output() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/above-threshold.md"),
        "# Test\nUsing [[tools]] in a semantic validation happy path.\n",
    );

    let embedding = deterministic_embedding(0.75);
    insert_blob_embedding(&env, "tool", &embedding);
    insert_blob_embedding(&env, "tools", &embedding);

    let output = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.99
        }),
    )
    .expect("above-threshold similarity should succeed");

    assert!(
        output.contains("All variants pass semantic similarity check"),
        "expected semantic validation success output, got: {output}"
    );
    assert!(
        output.contains("Scanning 1 markdown files"),
        "successful validation should continue into the repair flow: {output}"
    );
}

// T-14.9: RTM FR-6 / RTM-043 — validates that the semantic similarity gate works
// correctly with genuinely distinct embeddings whose cosine similarity is sub-1.0
// but above the configured threshold.  Seeds 0.75 and 0.50 produce vectors with
// cosine similarity ≈ 0.993, which passes a 0.99 threshold.  This closes the
// mid-range gap where tests 1–3 only exercise similarity 0.00 and 1.0.
#[test]
fn above_threshold_with_real_cosine_distance() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/cosine-midrange.md"),
        "# Test\nUsing [[tools]] with distinct but similar embeddings.\n",
    );

    let canonical_embedding = deterministic_embedding(0.75);
    let variant_embedding = deterministic_embedding(0.50);
    insert_blob_embedding(&env, "tool", &canonical_embedding);
    insert_blob_embedding(&env, "tools", &variant_embedding);

    let output = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.99
        }),
    )
    .expect("distinct embeddings above threshold should succeed");

    assert!(
        output.contains("All variants pass semantic similarity check"),
        "expected semantic validation success for sub-1.0 cosine above threshold, got: {output}"
    );
    assert!(
        output.contains("Scanning 1 markdown files"),
        "successful validation should continue into the repair flow: {output}"
    );
    assert!(
        !output.contains("BLOCKED"),
        "above-threshold similarity should not produce BLOCKED warning: {output}"
    );
}
