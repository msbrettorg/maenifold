#[path = "incremental_sync_unification/support.rs"]
mod incremental_sync_unification_support;

use chrono::Duration;
use incremental_sync_unification_support::*;

// T-5.7: RTM FR-14.1
#[test]
fn incremental_sync_mtime_guard_skips_unchanged_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-MtA-{unique}");
    let concept_common = format!("Incremental-Unify-MtCommon-{unique}");
    let content =
        format!("# Mtime Guard Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    let row_after_create = get_file_content_row(&cfg, &memory_uri).expect("indexed row");
    let stored_mtime = parse_rfc3339_utc(
        row_after_create
            .last_indexed
            .as_deref()
            .expect("stored last_indexed"),
    );
    set_file_timestamp(&file_path, stored_mtime);

    tamper_delete_mentions_and_edges_for_file(&cfg, &memory_uri);
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "mentions should be tampered away before re-sync"
    );

    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("mtime guard"),
        "same file mtime should skip reprocessing: {output}"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "when mtime is unchanged, sync_single_file must skip and must not restore missing mentions"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalized_concept(&concept_a),
            &normalized_concept(&concept_common),
        )
        .is_none(),
        "when mtime is unchanged, sync_single_file must not restore missing graph edges"
    );
}

// T-5.7: RTM FR-14.2
#[test]
fn incremental_sync_hash_guard_updates_last_indexed_only() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-HgA-{unique}");
    let concept_common = format!("Incremental-Unify-HgCommon-{unique}");
    let content =
        format!("# Hash Guard Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    let before = get_file_content_row(&cfg, &memory_uri).expect("indexed row");
    let original_hash = before.file_md5.clone().expect("stored hash");
    let previous_last_indexed =
        parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"));

    tamper_delete_mentions_and_edges_for_file(&cfg, &memory_uri);
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "mentions should be tampered away before re-sync"
    );

    let new_mtime = previous_last_indexed + Duration::minutes(1);
    set_file_timestamp(&file_path, new_mtime);

    let output = sync_path(&cfg, &file_path);
    let after = get_file_content_row(&cfg, &memory_uri).expect("updated row");

    assert!(
        output.contains("hash guard"),
        "same content should skip on hash guard: {output}"
    );
    assert_ne!(
        after.last_indexed, before.last_indexed,
        "when mtime differs but hash is unchanged, sync_single_file must update last_indexed"
    );
    assert_eq!(
        after.file_md5.as_deref(),
        Some(original_hash.as_str()),
        "when content hash is unchanged, sync_single_file must not update file_md5"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "when mtime differs but hash is unchanged, sync_single_file must not restore missing mentions"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalized_concept(&concept_a),
            &normalized_concept(&concept_common),
        )
        .is_none(),
        "when mtime differs but hash is unchanged, sync_single_file must not restore missing graph edges"
    );
}
