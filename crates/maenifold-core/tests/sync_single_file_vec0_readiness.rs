//! T-15.20: RTM FR-8 — verify `sync_single_file` checks vec0 extension availability
//! before setting `vector_ready=true`, matching the `run_sync` pattern and C#
//! `TryEnsureVectorSupport` contract.

#[path = "sync_unification/support.rs"]
mod sync_unification_support;

use maenifold_core::markdown::normalize_concept;
use sync_unification_support::*;

/// T-15.20: RTM FR-8 — when vec0 extension is not available, `sync_single_file` must still
/// complete successfully with concepts, mentions, and graph relations intact. The absence of
/// vec0 must not cause a crash or partial sync; embeddings are simply skipped.
#[test]
fn sync_single_file_without_vec0_processes_file_successfully() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    // The isolated temp dir has no vec0 extension in its assets path,
    // so vec0_extension_available() will return false.
    let concept = "vec0-readiness-alpha";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "vec0-readiness.md",
        &format!("# Vec0 Readiness Test\n\nReferences [[{concept}]].\n"),
    );

    let output = sync_one(&cfg, &path);

    // T-15.20: RTM FR-8 — sync should complete, not crash
    assert!(
        output.contains("Synced memory://tests/sync-unification/vec0-readiness"),
        "sync_single_file must succeed without vec0: {output}"
    );
    assert!(
        output.contains("concept mentions updated"),
        "sync must report mention processing: {output}"
    );

    // T-15.20: RTM FR-8 — file content and mentions must be correctly indexed
    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "file_content row must exist after sync_single_file without vec0"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept)),
        Some(1),
        "concept_mentions must contain concept with count 1 even without vec0"
    );
}

/// T-15.20: RTM FR-8 — when vec0 is not available, vector_ready must be false and embedding
/// generation must be skipped. Verify by confirming that the sync doesn't attempt vec0 table
/// operations that would fail on the connection. A second sync with changed content must also
/// succeed, proving that the full lifecycle works without vec0.
#[test]
fn sync_single_file_without_vec0_handles_content_updates() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "vec0-update-alpha";
    let concept_b = "vec0-update-beta";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "vec0-update.md",
        &format!("# Vec0 Update Test\n\nReferences [[{concept_a}]].\n"),
    );

    // Initial sync
    let output_initial = sync_one(&cfg, &path);
    assert!(
        output_initial.contains("Synced"),
        "initial sync must succeed: {output_initial}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
    );

    // Update content with a new concept and bump mtime
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");
    std::fs::write(
        &path,
        format!("# Vec0 Update Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    )
    .expect("rewrite content");
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + chrono::Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    // Re-sync with updated content — must succeed even without vec0
    let output_updated = sync_one(&cfg, &path);
    assert!(
        output_updated.contains("concept mentions updated"),
        "re-sync must process updated content without vec0: {output_updated}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "concept_a must remain after update"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        Some(1),
        "concept_b must appear after update even without vec0"
    );
}
