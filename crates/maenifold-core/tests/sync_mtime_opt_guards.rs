// T-12.0: RTM NFR-1
//
// These tests cover the three decision branches of `sync_single_file()`'s two-tier
// optimization gate. Unique contributions vs sibling files:
//   - Test 1: `UnreadableFileGuard` proves mtime guard fires *before* filesystem read —
//     no other test file verifies this behavioral guarantee.
//   - Test 2: Asserts `file_md5` and `file_size` preservation through the hash guard —
//     `sync_unification_single_file.rs` does not check these columns.
//   - Test 3: Asserts graph-edge removal and creation through full reprocessing —
//     verifies `rebuild_affected_graph_from_mentions` at the edge level.
#[path = "sync_mtime_opt/support.rs"]
mod support;

use chrono::Duration;
use support::{
    ConfigOverrideReset, SKIP_HASH_GUARD_MARKER, SKIP_MTIME_GUARD_MARKER, SYNCED_MENTIONS_MARKER,
    acquire_test_lock, acquire_unreadable_file_guard, create_test_file, get_edge_source_files,
    get_file_content_row, get_mention_count, isolated_config, normalized, parse_rfc3339_utc,
    set_file_mtime_utc, sync_one, tamper_delete_mentions_and_edges_for_file,
};

// T-12.0: RTM NFR-1
#[test]
fn process_file_changed_when_mtime_unchanged_skips_before_read_and_does_not_restore_state() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "Incremental-Mtime-ConceptA";
    let concept_common = "Incremental-Mtime-ConceptCommon";
    let content = format!(
        "# Incremental Mtime Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n"
    );
    let (path, file_uri) = create_test_file(&cfg, &content);

    let initial = sync_one(&cfg, &path);
    assert!(
        initial.contains("Synced memory://tests/incremental-mtime/note"),
        "initial sync should index the file: {initial}"
    );

    // Capture file_content row state before the mtime guard fires.
    let row_before_guard = get_file_content_row(&cfg, &file_uri).expect("row before guard");
    let stored_mtime = parse_rfc3339_utc(
        row_before_guard
            .last_indexed
            .as_deref()
            .expect("stored last_indexed"),
    );
    set_file_mtime_utc(&path, stored_mtime);

    tamper_delete_mentions_and_edges_for_file(&cfg, &file_uri);
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None
    );

    // The file is made unreadable — if sync_single_file attempts a read it will error.
    // Success proves the mtime guard fires before the filesystem read.
    {
        let _unreadable = acquire_unreadable_file_guard(&path);
        let output = sync_one(&cfg, &path);
        assert!(
            output.contains(SKIP_MTIME_GUARD_MARKER),
            "unchanged mtime should skip before read: {output}"
        );
    }

    // Verify file_content row was not modified (no DB writes on mtime-skip path).
    let row_after_guard = get_file_content_row(&cfg, &file_uri).expect("row after guard");
    assert_eq!(
        row_after_guard.last_indexed, row_before_guard.last_indexed,
        "mtime guard must not modify last_indexed"
    );
    assert_eq!(
        row_after_guard.file_md5, row_before_guard.file_md5,
        "mtime guard must not modify file_md5"
    );
    assert_eq!(
        row_after_guard.file_size, row_before_guard.file_size,
        "mtime guard must not modify file_size"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None,
        "mtime guard must not restore missing mentions"
    );
    assert_eq!(
        get_edge_source_files(&cfg, &normalized(concept_a), &normalized(concept_common)),
        None,
        "mtime guard must not restore missing graph edges"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn process_file_changed_when_mtime_differs_but_hash_same_updates_last_indexed_only() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "Incremental-Mtime-ConceptA";
    let concept_common = "Incremental-Mtime-ConceptCommon";
    let content = format!(
        "# Incremental Mtime Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n"
    );
    let (path, file_uri) = create_test_file(&cfg, &content);

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");

    tamper_delete_mentions_and_edges_for_file(&cfg, &file_uri);
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None
    );

    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);
    assert!(
        output.contains(SKIP_HASH_GUARD_MARKER),
        "mtime-only changes should skip reprocessing on identical content: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    // Compare against the pre-computed mtime (not a re-read of the filesystem) to avoid TOCTOU.
    assert_eq!(
        after.last_indexed.as_deref(),
        Some(new_mtime.to_rfc3339().as_str()),
        "hash guard must refresh last_indexed to the filesystem mtime"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "hash guard must preserve the stored checksum"
    );
    assert_eq!(
        after.file_size, before.file_size,
        "hash guard must preserve the stored file size"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None,
        "hash guard must not restore missing mentions"
    );
    assert_eq!(
        get_edge_source_files(&cfg, &normalized(concept_a), &normalized(concept_common)),
        None,
        "hash guard must not restore missing graph edges"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn process_file_changed_when_hash_differs_reprocesses_and_updates_mentions() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "Incremental-Mtime-ConceptA";
    let concept_common = "Incremental-Mtime-ConceptCommon";
    let concept_b = "Incremental-Mtime-ConceptB";
    let content_v1 = format!(
        "# Incremental Mtime Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n"
    );
    let (path, file_uri) = create_test_file(&cfg, &content_v1);

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");

    tamper_delete_mentions_and_edges_for_file(&cfg, &file_uri);
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None
    );

    let content_v2 = format!(
        "# Incremental Mtime Test\n\nNow references [[{concept_b}]] and [[{concept_common}]] instead.\n"
    );
    std::fs::write(&path, &content_v2).expect("rewrite content");
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);
    assert!(
        output.contains(SYNCED_MENTIONS_MARKER),
        "content changes should trigger reprocessing: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    // Compare against the pre-computed mtime (not a re-read of the filesystem) to avoid TOCTOU.
    assert_eq!(
        after.last_indexed.as_deref(),
        Some(new_mtime.to_rfc3339().as_str()),
        "reprocessing must set last_indexed to the filesystem mtime"
    );
    assert_ne!(
        after.file_md5, before.file_md5,
        "reprocessing must update the checksum when content changes"
    );
    assert_ne!(
        after.file_size, before.file_size,
        "reprocessing must update the stored file size when content changes"
    );

    // Mention-layer assertions.
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_a)),
        None,
        "reprocessing must remove mentions for concepts no longer present"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_b)),
        Some(1),
        "reprocessing must add mentions for newly present concepts"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalized(concept_common)),
        Some(1),
        "reprocessing must restore mentions for concepts still present"
    );

    // Graph-edge assertions — the most expensive/complex part of reprocessing.
    // Old edge removed: concept_a is no longer mentioned alongside concept_common.
    assert_eq!(
        get_edge_source_files(&cfg, &normalized(concept_a), &normalized(concept_common)),
        None,
        "reprocessing must remove the edge for a concept no longer mentioned"
    );
    // New edge created: concept_b now co-occurs with concept_common.
    assert!(
        get_edge_source_files(&cfg, &normalized(concept_b), &normalized(concept_common)).is_some(),
        "reprocessing must create an edge for newly co-occurring concepts"
    );
}
