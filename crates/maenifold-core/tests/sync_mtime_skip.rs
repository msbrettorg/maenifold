#[path = "sync_mtime/support.rs"]
mod sync_mtime_support;

use sync_mtime_support::{
    ConfigOverrideReset, TEST_MUTEX, create_test_markdown_file, file_mtime, file_mtime_rfc3339,
    get_file_content_row, get_mention_count, isolated_env, mtime_plus, set_file_mtime_system_time,
    sync_all, tamper_db_delete_mentions,
};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
use sync_mtime_support::PermissionRestore;

// T-2.3: RTM FR-14.1 - Ports SyncMtimeOptimizationTests.Sync_WhenMtimeUnchanged_SkipsWithoutRead_AndDoesNotRestoreMentionsOrEdges_OnUnix.
// Strategy: chmod(0) serves as a read-impossibility oracle — if the mtime guard fails to skip,
// sync attempts to read the file and surfaces an I/O error rather than silently passing.
#[cfg(unix)]
#[test]
fn sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "unchanged-mtime.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let original_mtime = file_mtime(&file_path);
    let before = get_file_content_row(&env.cfg, &memory_uri)
        .expect("indexed row should exist after initial sync");
    let original_last_indexed = before
        .last_indexed
        .clone()
        .expect("last_indexed should be set after initial sync");
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), Some(1));
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), Some(1));

    tamper_db_delete_mentions(&env.cfg, &memory_uri);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), None);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), None);

    set_file_mtime_system_time(&file_path, original_mtime);

    let original_mode = fs::metadata(&file_path)
        .expect("metadata before chmod")
        .permissions()
        .mode();
    let _restore = PermissionRestore {
        path: file_path.clone(),
        mode: original_mode,
    };
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0)).expect("make file unreadable");

    assert_eq!(
        file_mtime(&file_path),
        original_mtime,
        "making file unreadable must not change mtime"
    );

    // Guard: if the file is still readable (e.g. running as root in a Docker container),
    // the chmod oracle is ineffective — skip the test rather than giving a false pass.
    if std::fs::read(&file_path).is_ok() {
        eprintln!(
            "SKIP: chmod(0) did not make the file unreadable (likely running as root); \
             the read-impossibility oracle cannot function"
        );
        return;
    }

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "unchanged mtime should be skipped without surfacing a read error: {output}"
    );

    let after = get_file_content_row(&env.cfg, &memory_uri)
        .expect("row should still exist after sync");
    assert_eq!(
        after.last_indexed,
        Some(original_last_indexed),
        "unchanged mtime should leave last_indexed untouched"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "unchanged mtime should leave file_md5 untouched"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_a),
        None,
        "unchanged mtime should skip processing without restoring mentions"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_b),
        None,
        "unchanged mtime should skip processing without restoring mentions"
    );
}

// T-2.3: RTM FR-14.2 - Ports SyncMtimeOptimizationTests.Sync_WhenMtimeBumpedButContentSame_UpdatesLastIndexedOnly_AndDoesNotRestoreMentionsOrEdges.
#[test]
fn sync_when_mtime_bumped_but_content_same_updates_last_indexed_only_and_does_not_restore_mentions()
{
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "same-content-new-mtime.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let before = get_file_content_row(&env.cfg, &memory_uri).expect("initial file_content row");
    tamper_db_delete_mentions(&env.cfg, &memory_uri);

    let desired_mtime = mtime_plus(&file_path, 120);
    set_file_mtime_system_time(&file_path, desired_mtime);
    let actual_new_mtime = file_mtime_rfc3339(&file_path);

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "sync should succeed on hash-guard path: {output}"
    );

    let after = get_file_content_row(&env.cfg, &memory_uri).expect("updated file_content row");
    assert_eq!(
        after.last_indexed,
        Some(actual_new_mtime),
        "when content hash matches, sync should update last_indexed to the filesystem mtime"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "when content hash matches, sync should preserve file_md5"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_a),
        None,
        "hash guard should not restore mentions"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_b),
        None,
        "hash guard should not restore mentions"
    );
}

// T-2.3: RTM FR-14.3 - Ports SyncMtimeOptimizationTests.Sync_WhenContentChanged_ReprocessesAndRestoresMentionsAndEdges_IncludingNewConcept.
#[test]
fn sync_when_content_changed_reprocesses_and_restores_mentions_including_new_concept() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let concept_c = "sync-mtime-c";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "content-changed.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let before = get_file_content_row(&env.cfg, &memory_uri).expect("initial file_content row");
    tamper_db_delete_mentions(&env.cfg, &memory_uri);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), None);

    std::fs::write(
        &file_path,
        format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]] and now [[{concept_c}]].\n"
        ),
    )
    .expect("rewrite fixture");
    // Explicitly advance mtime to ensure the delta exceeds filesystem timestamp resolution
    // (e.g. HFS+ 1-second granularity). fs::write updates mtime automatically, but the
    // resulting mtime may land within the same second as the baseline, causing a false skip.
    set_file_mtime_system_time(&file_path, mtime_plus(&file_path, 120));
    let new_mtime = file_mtime_rfc3339(&file_path);

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "content change should be reprocessed successfully: {output}"
    );

    let after = get_file_content_row(&env.cfg, &memory_uri).expect("updated file_content row");
    assert_eq!(
        after.last_indexed,
        Some(new_mtime),
        "content change should update last_indexed to the filesystem mtime"
    );
    assert_ne!(
        after.file_md5, before.file_md5,
        "content change must update file_md5"
    );
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), Some(1));
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), Some(1));
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_c),
        Some(1),
        "processing must restore mentions and include the new concept"
    );
}
