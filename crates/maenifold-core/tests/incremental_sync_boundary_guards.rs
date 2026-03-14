#[path = "incremental_sync_boundary/support.rs"]
mod support;

// T-12.1: RTM NFR-1
use support::{
    ConfigOverrideReset, acquire_test_lock, baseline_sync, concept_names, fetch_file_md5,
    fetch_last_indexed, file_concepts, file_uri, isolated_config, set_last_indexed, sync_one,
    write_memory_fixture,
};

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_skips_unchanged_file_with_mtime_guard() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "notes/mtime-guard.md", "# Note\n[[alpha]] [[beta]]\n");

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/mtime-guard.md");
    let before_last_indexed = fetch_last_indexed(&cfg, &target_uri).expect("indexed");
    let before_concepts = concept_names(&cfg);

    let output = sync_one(&cfg, "notes/mtime-guard.md");

    assert!(
        output.contains("mtime guard"),
        "unchanged file should skip on mtime guard: {output}"
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &target_uri),
        Some(before_last_indexed),
        "mtime guard should not update last_indexed"
    );
    assert_eq!(
        concept_names(&cfg),
        before_concepts,
        "mtime guard should leave graph state unchanged"
    );
    // T-5.7: RTM FR-5.7 — pin exact file-scoped concept set (not just whole-DB snapshot)
    let expected: std::collections::BTreeSet<String> =
        ["alpha", "beta"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        file_concepts(&cfg, &target_uri),
        expected,
        "mtime guard should preserve the exact file-scoped concept set"
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_skips_file_with_same_hash() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let content = "# Hash Guard\n[[alpha]] [[shared]]\n";
    write_memory_fixture(&cfg, "notes/hash-guard.md", content);

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/hash-guard.md");
    let old_timestamp = "2000-01-01T00:00:00+00:00";
    let original_hash = fetch_file_md5(&cfg, &target_uri).expect("stored hash");
    let before_concepts = concept_names(&cfg);
    set_last_indexed(&cfg, &target_uri, old_timestamp);
    write_memory_fixture(&cfg, "notes/hash-guard.md", content);

    let output = sync_one(&cfg, "notes/hash-guard.md");

    assert!(
        output.contains("hash guard"),
        "same content should skip on hash guard: {output}"
    );
    let updated_last_indexed = fetch_last_indexed(&cfg, &target_uri).expect("updated indexed_at");
    assert_ne!(
        updated_last_indexed, old_timestamp,
        "hash guard should refresh last_indexed after re-check"
    );
    // T-5.7: RTM FR-5.7 — verify refreshed timestamp is a plausible current-year RFC-3339 value
    assert!(
        updated_last_indexed.starts_with("202"),
        "hash guard should refresh last_indexed to current time, got: {updated_last_indexed}"
    );
    assert_eq!(
        fetch_file_md5(&cfg, &target_uri),
        Some(original_hash),
        "hash guard should preserve stored checksum"
    );
    assert_eq!(
        concept_names(&cfg),
        before_concepts,
        "hash guard should not rebuild unrelated concepts"
    );
}
