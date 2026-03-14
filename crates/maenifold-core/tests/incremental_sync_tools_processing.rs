#[path = "incremental_sync_tools/support.rs"]
mod support;

use support::{
    ConfigOverrideReset, TEST_MUTEX, context, count_rows_like, create_test_file, get_file_md5,
    get_file_row, get_mention_count, isolated_config, normalize_concept, path_to_uri,
    process_path_for_test, process_pending_paths_for_test, set_last_indexed,
};

// T-12.7: RTM NFR-1 - Split incremental sync processing assertions into a focused file.

// T-12.7: RTM NFR-2 - This test exercises `process_path_for_test` directly, bypassing leader
// election and the higher-level `sync_one` path. The unification suite
// (`sync_unification_single_file::process_file_new_file_inserts_content_mentions_and_index_rows`)
// covers the same DB assertions via `sync_one`. This test is retained to verify the lower-level
// entry-point produces identical observable state.
#[test]
fn process_file_created_with_wikilinks_inserts_concept_mentions_into_db() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept_a = format!(
        "incremental-sync-created-a-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_b = format!(
        "incremental-sync-created-b-{}",
        uuid::Uuid::new_v4().simple()
    );
    let content = format!("# Test Note\n\nThis references [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, "tests/incremental-sync-tools", &content);

    process_path_for_test(&ctx, &file_path).expect("process created file");

    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
        Some(1)
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)),
        Some(1)
    );
    assert_eq!(get_file_row(&cfg, &memory_uri), Some(memory_uri));
}

#[test]
fn process_file_changed_with_new_content_adds_mention_for_new_concept_and_updates_file_content() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept_a = format!(
        "incremental-sync-changed-a-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_b = format!(
        "incremental-sync-changed-b-{}",
        uuid::Uuid::new_v4().simple()
    );
    let content_v1 = format!("# Test Note\n\nThis references [[{concept_a}]].\n");
    let (file_path, memory_uri) =
        create_test_file(&cfg, "tests/incremental-sync-tools", &content_v1);

    process_path_for_test(&ctx, &file_path).expect("process created file");
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
        Some(1)
    );

    let hash_after_create = get_file_md5(&cfg, &memory_uri).expect("stored hash after create");
    set_last_indexed(&cfg, &memory_uri, "2000-01-01T00:00:00+00:00");
    let content_v2 = format!("# Test Note\n\nThis now also references [[{concept_b}]].\n");
    std::fs::write(&file_path, content_v2).expect("rewrite fixture");

    process_path_for_test(&ctx, &file_path).expect("process changed file");

    // T-12.7: RTM NFR-1 - P0: Assert the old concept was removed after content update.
    // Without this, a bug that appends rather than reconciles concept mentions would go undetected.
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
        None,
        "concept_a should be removed after content update that no longer references it"
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)),
        Some(1),
        "concept_b should be present after content update"
    );
    assert_ne!(
        get_file_md5(&cfg, &memory_uri).expect("stored hash after change"),
        hash_after_create,
        "file MD5 should change after content update"
    );
}

#[test]
fn process_file_deleted_after_create_removes_file_content_and_mentions_from_db() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept = format!("incremental-sync-deleted-{}", uuid::Uuid::new_v4().simple());
    let content = format!("# Test Note\n\nThis references [[{concept}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, "tests/incremental-sync-tools", &content);

    process_path_for_test(&ctx, &file_path).expect("process created file");
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept)),
        Some(1)
    );

    std::fs::remove_file(&file_path).expect("delete fixture");
    process_path_for_test(&ctx, &file_path).expect("process deleted file");

    assert_eq!(get_file_row(&cfg, &memory_uri), None);
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept)),
        None
    );
}

// T-12.7: RTM NFR-2 - Verify batch processing only indexes markdown files, ignoring other types.
#[test]
fn watcher_processing_ignores_non_markdown_files_in_batch() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let nested = cfg.memory_path.join("tests/incremental-sync-tools/nested");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    let ignored_txt = nested.join("ignored.txt");
    let ignored_json = nested.join("ignored.json");
    let watched_md = nested.join("watched.md");
    std::fs::write(&ignored_txt, "[[ignored-concept-txt]]\n").expect("write txt");
    std::fs::write(
        &ignored_json,
        "{\"note\": \"[[ignored-concept-json]]\"}\n",
    )
    .expect("write json");
    std::fs::write(&watched_md, "# Watched\n\n[[watched-concept]]\n").expect("write markdown");
    let watched_uri = path_to_uri(&watched_md, &cfg.memory_path).expect("watched uri");

    process_pending_paths_for_test(
        &ctx,
        &[ignored_txt.clone(), ignored_json.clone(), watched_md.clone()],
    )
    .expect("process pending paths");

    assert_eq!(get_file_row(&cfg, &watched_uri), Some(watched_uri));
    assert_eq!(
        count_rows_like(&cfg, "file_content", "%ignored.txt%"),
        0,
        ".txt files should not be indexed"
    );
    assert_eq!(
        count_rows_like(&cfg, "file_content", "%ignored.json%"),
        0,
        ".json files should not be indexed"
    );
}

// T-12.7: RTM NFR-1 - P0: Verify path deduplication coalesces rapid changes into a single
// final-state sync. concept_v1 is indexed first so that asserting its removal is meaningful.
#[test]
fn debounce_rapid_changes_to_same_path_coalesces_into_final_processing() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept_v1 = format!(
        "incremental-sync-debounce-v1-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_final = format!(
        "incremental-sync-debounce-final-{}",
        uuid::Uuid::new_v4().simple()
    );
    let (file_path, memory_uri) = create_test_file(
        &cfg,
        "tests/incremental-sync-tools",
        &format!("# Version 1\n\n[[{concept_v1}]]\n"),
    );

    // Index the initial state so concept_v1 is actually in the DB
    process_path_for_test(&ctx, &file_path).expect("index initial v1");
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_v1)),
        Some(1),
        "precondition: concept_v1 must be indexed before dedup test"
    );

    // Force re-processing by backdating last_indexed
    set_last_indexed(&cfg, &memory_uri, "2000-01-01T00:00:00+00:00");

    // Apply rapid overwrites — simulates debounced file-system events
    let mut pending_events = Vec::new();
    for version in 0..5 {
        std::fs::write(
            &file_path,
            format!("# Final Version {version}\n\n[[{concept_final}]]\n"),
        )
        .expect("write debounced content");
        pending_events.push(file_path.clone());
    }

    process_pending_paths_for_test(&ctx, &pending_events).expect("process debounced paths");

    // concept_v1 was in the DB and should have been reconciled away by the update
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_v1)),
        None,
        "concept_v1 should be reconciled away after rapid overwrites"
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_final)),
        Some(1),
        "concept_final should be the only concept after deduplication"
    );
}
