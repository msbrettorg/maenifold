#[path = "incremental_sync_boundary/support.rs"]
mod support;

// T-12.1: RTM NFR-1
use std::collections::BTreeSet;
use std::fs;
use support::{
    ConfigOverrideReset, acquire_test_lock, baseline_sync, concept_names, concept_sources,
    fetch_last_indexed, file_concepts, file_uri, isolated_config, set_last_indexed, sync_one,
    write_memory_fixture,
};

// T-5.7: RTM FR-14.3
#[test]
fn sync_single_file_processes_a_new_file_correctly() {
    let _guard = acquire_test_lock();
    // ConfigOverrideReset restores Config::current() for the next test on drop.
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "notes/baseline.md",
        "# Baseline\n[[seed]] [[shared]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let baseline_uri = file_uri("notes/baseline.md");
    let baseline_last_indexed = fetch_last_indexed(&cfg, &baseline_uri).expect("baseline indexed");

    write_memory_fixture(&cfg, "notes/new-file.md", "# New\n[[fresh]] [[shared]]\n");
    let output = sync_one(&cfg, "notes/new-file.md");

    assert!(
        output.contains("Synced memory://notes/new-file"),
        "new file should be processed: {output}"
    );
    // Verify graph state structurally rather than checking output string counts.
    assert_eq!(
        file_concepts(&cfg, &file_uri("notes/new-file.md")),
        BTreeSet::from(["fresh".to_string(), "shared".to_string()]),
        "new file should contribute its concepts to the graph"
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from([
            "fresh".to_string(),
            "seed".to_string(),
            "shared".to_string()
        ])
    );
    assert_eq!(
        concept_sources(&cfg, "fresh"),
        BTreeSet::from(["memory://notes/new-file".to_string()]),
        "fresh concept should be sourced only from the new file"
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &baseline_uri),
        Some(baseline_last_indexed),
        "unchanged baseline file should not be reprocessed"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn sync_single_file_updates_concepts_when_content_changes() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "notes/target.md", "# Target\n[[alpha]] [[shared]]\n");
    write_memory_fixture(
        &cfg,
        "notes/untouched.md",
        "# Untouched\n[[shared]] [[stable]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/target.md");
    let untouched_uri = file_uri("notes/untouched.md");
    let untouched_last_indexed =
        fetch_last_indexed(&cfg, &untouched_uri).expect("untouched last_indexed");
    // Back-date the DB record so the mtime guard sees file_mtime > last_indexed
    // and classifies the rewritten file as changed, triggering reprocessing.
    set_last_indexed(&cfg, &target_uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(&cfg, "notes/target.md", "# Target\n[[beta]] [[shared]]\n");

    let output = sync_one(&cfg, "notes/target.md");

    assert!(
        output.contains("Synced memory://notes/target"),
        "changed file should be processed: {output}"
    );
    assert_eq!(
        file_concepts(&cfg, &target_uri),
        BTreeSet::from(["beta".to_string(), "shared".to_string()])
    );
    assert!(
        concept_sources(&cfg, "alpha").is_empty(),
        "alpha should have no remaining mentions: {:?}",
        concept_sources(&cfg, "alpha")
    );
    assert_eq!(
        concept_sources(&cfg, "beta"),
        BTreeSet::from(["memory://notes/target".to_string()]),
        "beta should be sourced only from the target file"
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from([
            "beta".to_string(),
            "shared".to_string(),
            "stable".to_string()
        ]),
        "old concept should be removed and untouched concepts preserved"
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &untouched_uri),
        Some(untouched_last_indexed),
        "unchanged file should not be reprocessed during single-file sync"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn deleted_files_remove_their_concepts_from_the_graph() {
    let _guard = acquire_test_lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "notes/deleted.md",
        "# Deleted\n[[alpha]] [[shared]]\n",
    );
    write_memory_fixture(
        &cfg,
        "notes/survivor.md",
        "# Survivor\n[[shared]] [[stable]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    fs::remove_file(cfg.memory_path.join("notes/deleted.md")).expect("delete target file");
    let output = sync_one(&cfg, "notes/deleted.md");

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed concepts: {output}"
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from(["shared".to_string(), "stable".to_string()]),
        "deleted file's unique concepts should be removed"
    );
    assert!(
        file_concepts(&cfg, &file_uri("notes/deleted.md")).is_empty(),
        "deleted file should have no remaining concept mentions"
    );
    assert!(
        concept_sources(&cfg, "alpha").is_empty(),
        "deleted alpha concept should have no remaining mentions: {:?}",
        concept_sources(&cfg, "alpha")
    );
    assert_eq!(
        concept_sources(&cfg, "shared"),
        BTreeSet::from(["memory://notes/survivor".to_string()]),
        "shared concept's mention in survivor file must survive deletion of peer"
    );
}
