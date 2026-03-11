#[path = "sync_unification/support.rs"]
mod sync_unification_support;

use maenifold_core::markdown::normalize_concept;
use sync_unification_support::*;

// T-5.7: RTM FR-14.3
#[test]
fn remove_file_cleans_content_and_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "remove-file.md",
        &format!("# Removal Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    sync_one(&cfg, &path);
    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "precondition: file_content must exist"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "precondition: mentions must exist"
    );

    std::fs::remove_file(&path).expect("delete target file");
    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed concepts: {output}"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri).is_none(),
        "file_content row must be deleted after syncing a removed file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "concept_mentions must be deleted after removing the file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        None,
        "concept_mentions must be deleted after removing the file"
    );
}

// T-5.7: RTM FR-14.3
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

    assert!(
        output.contains("Sync complete:"),
        "batch sync should complete successfully: {output}"
    );
    assert!(
        output.contains("- 3 FILES processed"),
        "batch sync should process all files in the fixture set: {output}"
    );
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
}

// T-5.7: RTM FR-14.3
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
