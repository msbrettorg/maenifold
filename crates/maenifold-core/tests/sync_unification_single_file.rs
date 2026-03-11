#[path = "sync_unification/support.rs"]
mod sync_unification_support;

use maenifold_core::markdown::normalize_concept;
use sync_unification_support::*;

// T-5.7: RTM FR-14.3
#[test]
fn process_file_new_file_inserts_content_mentions_and_index_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "new-file.md",
        &format!("# Unification Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("Synced memory://tests/sync-unification/new-file"),
        "new file should be processed: {output}"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "file_content row must exist after sync_single_file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "concept_mentions must contain concept_a with count 1"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        Some(1),
        "concept_mentions must contain concept_b with count 1"
    );
}

// T-5.7: RTM FR-14.1
#[test]
fn process_file_mtime_unchanged_skips_without_restoring_deleted_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "mtime-unchanged.md",
        &format!("# Skip Test\n\nReferences [[{concept_a}]].\n"),
    );

    let initial = sync_one(&cfg, &path);
    assert!(
        initial.contains("Synced memory://tests/sync-unification/mtime-unchanged"),
        "initial sync should index the file: {initial}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "precondition: mention must exist after initial sync"
    );

    tamper_delete_mentions(&cfg, &file_uri);
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "precondition: mentions must be absent after tamper"
    );

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("mtime guard"),
        "mtime guard should skip unchanged file: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "mentions must remain deleted when mtime is unchanged"
    );
}

// T-5.7: RTM FR-14.2
#[test]
fn process_file_mtime_differs_but_hash_same_updates_last_indexed_only() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "hash-guard.md",
        &format!("# Hash Guard Test\n\nReferences [[{concept_a}]].\n"),
    );

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");

    tamper_delete_mentions(&cfg, &file_uri);
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + chrono::Duration::minutes(2);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("hash guard"),
        "mtime-only changes should skip reprocessing on identical content: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_eq!(
        after.last_indexed,
        Some(file_mtime_rfc3339(&path)),
        "last_indexed must update to the new filesystem mtime"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "file_md5 must remain unchanged when content is identical"
    );
    assert_eq!(
        after.file_size, before.file_size,
        "file_size must remain unchanged when content is identical"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "mentions must remain deleted when the hash guard skips reprocessing"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn process_file_content_changed_reprocesses_all_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let concept_c = "sync-unify-c";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "content-change.md",
        &format!("# Content Change Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1)
    );

    std::fs::write(
        &path,
        format!(
            "# Content Change Test\n\nReferences [[{concept_a}]], [[{concept_b}]], and [[{concept_c}]].\n"
        ),
    )
    .expect("rewrite content");
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + chrono::Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("concept mentions updated"),
        "content changes should trigger reprocessing: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_ne!(
        after.file_md5, before.file_md5,
        "file_md5 must change when file content changes"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1)
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        Some(1)
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_c)),
        Some(1),
        "new concept must appear in mentions after reprocessing"
    );
}
