// Batch-path (`sync_all`) tests for the unified sync pipeline.
// Single-file operations are tested in `sync_unification_single_file.rs`.
// Deletion-via-sync_single_file is covered by
// `incremental_sync_unification_lifecycle::incremental_sync_file_deleted_cleans_all_artifacts`.

#[path = "sync_unification/support.rs"]
mod sync_unification_support;

use maenifold_core::markdown::normalize_concept;
use sync_unification_support::*;

// T-5.7: FR archived — see CHANGELOG.md § FR-14 (RTM-080). Batch sync DB-coherence.
#[test]
fn sync_batch_processes_multiple_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let concept_c = "sync-unify-c";
    let (_path1, file_uri1) = create_test_markdown_file(
        &cfg,
        "batch-1.md",
        &format!("# Batch 1\n\nReferences [[{concept_a}]].\n"),
    );
    let (_path2, file_uri2) = create_test_markdown_file(
        &cfg,
        "batch-2.md",
        &format!("# Batch 2\n\nReferences [[{concept_b}]].\n"),
    );
    let (_path3, file_uri3) = create_test_markdown_file(
        &cfg,
        "batch-3.md",
        &format!("# Batch 3\n\nReferences [[{concept_c}]].\n"),
    );

    let output = sync_all(&cfg);

    // Primary contract: sync reports completion
    assert!(
        output.contains("Sync complete:"),
        "batch sync should complete successfully: {output}"
    );
    // Resilient count check: tolerate case variation in "FILES/files processed"
    let output_lower = output.to_lowercase();
    assert!(
        output_lower.contains("files processed") && output.contains("3"),
        "batch sync should process all files in the fixture set: {output}"
    );

    // DB-row assertions (primary contract — not format-dependent)
    assert!(
        get_file_content_row(&cfg, &file_uri1).is_some(),
        "file 1 must have a file_content row"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri2).is_some(),
        "file 2 must have a file_content row"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri3).is_some(),
        "file 3 must have a file_content row"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_a)),
        Some(1),
        "file 1 mentions must be recorded"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri2, &normalize_concept(concept_b)),
        Some(1),
        "file 2 mentions must be recorded"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri3, &normalize_concept(concept_c)),
        Some(1),
        "file 3 mentions must be recorded"
    );

    // Cross-file concept isolation: each file has exactly one concept,
    // so no co-occurrence graph edges should be created.
    assert_eq!(
        get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_b)),
        None,
        "file 1 must not reference file 2's concept"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_c)),
        None,
        "file 1 must not reference file 3's concept"
    );

    // No concept_graph edges should exist — single-concept-per-file means no co-occurrence.
    assert!(
        get_edge_source_files(
            &cfg,
            &normalize_concept(concept_a),
            &normalize_concept(concept_b)
        )
        .is_none(),
        "no graph edge between concept_a and concept_b (different files, no co-occurrence)"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalize_concept(concept_a),
            &normalize_concept(concept_c)
        )
        .is_none(),
        "no graph edge between concept_a and concept_c (different files, no co-occurrence)"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalize_concept(concept_b),
            &normalize_concept(concept_c)
        )
        .is_none(),
        "no graph edge between concept_b and concept_c (different files, no co-occurrence)"
    );
}

// T-5.7: FR archived — see CHANGELOG.md § FR-14 (RTM-080). Empty-directory edge case.
#[test]
fn sync_empty_memory_directory_returns_without_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "empty sync should still return a report: {output}"
    );
    assert!(
        output.contains("- 0 FILES processed"),
        "empty sync should report zero processed files: {output}"
    );
}
