// Unification lifecycle tests verify multi-table DB coherence (file_content, concept_mentions,
// concept_graph) for each sync_single_file() lifecycle transition: create, delete, rename.
// The complementary boundary suite (incremental_sync_boundary_mutations) verifies behavior
// contracts at a shallower level (concept existence, source sets). Both layers are needed:
// boundary tests catch behavioral regressions; unification tests catch DB-column-level bugs
// (e.g., missing file_size, orphaned concept_mentions surviving a delete, graph edges not
// rebuilt after rename).

#[path = "incremental_sync_unification/support.rs"]
mod incremental_sync_unification_support;

use incremental_sync_unification_support::*;
use maenifold_core::markdown::path_to_uri;
use std::fs;

// T-5.7: FR archived — see CHANGELOG.md § FR-14 (RTM-080). Incremental sync DB-coherence
// coverage; no current open RTM item — all relevant items delivered and archived.
#[test]
fn incremental_sync_new_file_creates_expected_db_artifacts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-Alpha-{unique}");
    let concept_b = format!("Incremental-Unify-Beta-{unique}");
    let content =
        format!("# Unification Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");

    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);
    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("2 concept mentions updated"),
        "incremental sync should index exactly 2 concept mentions: {output}"
    );

    let file_content = get_file_content_row(&cfg, &memory_uri);
    let mention_a = get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a));
    let mention_b = get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_b));

    assert!(
        file_content.is_some(),
        "incremental sync should create file_content row"
    );
    assert_eq!(
        mention_a,
        Some(1),
        "mention count for concept A should be 1"
    );
    assert_eq!(
        mention_b,
        Some(1),
        "mention count for concept B should be 1"
    );

    let file_content = file_content.expect("file_content");
    assert!(
        file_content
            .file_md5
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        "incremental sync must populate file_md5"
    );
    assert!(
        file_content.file_size.is_some_and(|value| value > 0),
        "incremental sync must populate file_size"
    );

    // Verify concept_graph co-occurrence edge was created between the two concepts.
    // Test 2 (delete) asserts graph edges are cleaned; symmetrically, the create path
    // must verify they are established.
    let edge = get_edge_source_files(
        &cfg,
        &normalized_concept(&concept_a),
        &normalized_concept(&concept_b),
    );
    assert!(
        edge.as_ref()
            .is_some_and(|files| files.contains_key(&memory_uri)),
        "concept_graph edge must exist and reference the indexed file's URI as a source"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn incremental_sync_file_deleted_cleans_all_artifacts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-DelA-{unique}");
    let concept_b = format!("Incremental-Unify-DelB-{unique}");
    let content = format!("# Delete Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    assert!(
        get_file_content_row(&cfg, &memory_uri).is_some(),
        "file should be indexed before deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_some(),
        "mention A should exist before deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_b)).is_some(),
        "mention B should exist before deletion"
    );

    fs::remove_file(&file_path).expect("delete file");
    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed state: {output}"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "concept_mentions for concept A must be removed after deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_b)).is_none(),
        "concept_mentions for concept B must be removed after deletion"
    );
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_none(),
        "file_content row must be removed after deletion"
    );

    let edge = get_edge_source_files(
        &cfg,
        &normalized_concept(&concept_a),
        &normalized_concept(&concept_b),
    );
    assert!(
        edge.as_ref()
            .is_none_or(|files| !files.contains_key(&memory_uri)),
        "concept_graph edge must not reference the deleted file"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn incremental_sync_file_renamed_reindexes_under_new_uri() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-RenA-{unique}");
    let concept_b = format!("Incremental-Unify-RenB-{unique}");
    let content = format!("# Rename Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_some(),
        "original file should be indexed"
    );

    let test_folder = file_path.parent().expect("parent").to_path_buf();
    let new_file_path = test_folder.join(format!("renamed-{unique}.md"));
    fs::rename(&file_path, &new_file_path).expect("rename file");
    let new_memory_uri = path_to_uri(&new_file_path, &cfg.memory_path).expect("new uri");

    let delete_output = sync_path(&cfg, &file_path);
    let create_output = sync_path(&cfg, &new_file_path);

    assert!(
        delete_output.contains("removed stale index entry"),
        "old path should be deleted after rename: {delete_output}"
    );
    assert!(
        create_output.contains("concept mentions updated"),
        "new path should be indexed after rename: {create_output}"
    );
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_none(),
        "old URI must be removed from file_content after rename"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalized_concept(&concept_a)).is_none(),
        "old URI concept_mentions must be removed after rename"
    );
    assert!(
        get_file_content_row(&cfg, &new_memory_uri).is_some(),
        "new URI must be present in file_content after rename"
    );
    assert!(
        get_mention_count(&cfg, &new_memory_uri, &normalized_concept(&concept_a)).is_some(),
        "new URI must have concept_mentions for concept A after rename"
    );
    assert!(
        get_mention_count(&cfg, &new_memory_uri, &normalized_concept(&concept_b)).is_some(),
        "new URI must have concept_mentions for concept B after rename"
    );
}
