// T-12.0: RTM NFR-1
#[path = "sync_mtime_opt/support.rs"]
mod support;

use chrono::Duration;
use support::{
    ConfigOverrideReset, acquire_test_lock, acquire_unreadable_file_guard, create_test_file,
    file_mtime_rfc3339, get_edge_source_files, get_file_content_row, get_mention_count,
    isolated_config, normalized, parse_rfc3339_utc, set_file_mtime_utc, sync_one,
    tamper_delete_mentions_and_edges_for_file,
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

    let row_after_create = get_file_content_row(&cfg, &file_uri).expect("row after create");
    let stored_mtime = parse_rfc3339_utc(
        row_after_create
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

    let original_mtime = parse_rfc3339_utc(&file_mtime_rfc3339(&path));
    {
        let _unreadable = acquire_unreadable_file_guard(&path);
        set_file_mtime_utc(&path, original_mtime);
        let output = sync_one(&cfg, &path);
        assert!(
            output.contains("mtime guard"),
            "unchanged mtime should skip before read: {output}"
        );
    }

    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "initial sync should keep the file_content row intact"
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
        output.contains("hash guard"),
        "mtime-only changes should skip reprocessing on identical content: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_eq!(
        after.last_indexed,
        Some(file_mtime_rfc3339(&path)),
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
    std::fs::write(&path, content_v2).expect("rewrite content");
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);
    assert!(
        output.contains("concept mentions updated"),
        "content changes should trigger reprocessing: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_eq!(
        after.last_indexed,
        Some(file_mtime_rfc3339(&path)),
        "reprocessing must set last_indexed to the filesystem mtime"
    );
    assert_ne!(
        after.file_md5, before.file_md5,
        "reprocessing must update the checksum when content changes"
    );
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
}
