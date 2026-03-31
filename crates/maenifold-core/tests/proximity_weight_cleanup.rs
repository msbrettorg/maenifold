// T-12.0: RTM NFR-1
#[allow(clippy::duplicate_mod)]
#[path = "proximity_weight/support.rs"]
mod support;

use std::fs;
use support::{
    TestEnv, assert_approx_eq, invoke_sync, open_db, relation_count,
    relation_exists, relation_source_files, write_memory_file,
};

// T-4.3: RTM FR-23.5 — NOTE: FR-23 in RTM.md is HTTP Transport (collision);
// proximity weight cleanup requires PM RTM reconciliation.
#[test]
fn sync_removes_edge_when_last_source_file_is_deleted() {
    let env = TestEnv::isolated();

    let relative = "tests/to-remove.md";
    write_memory_file(
        &env.cfg,
        relative,
        "# Remove\n\n## Identity\n[[alpha]] and [[beta]] live together.\n",
    );
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert!(
        relation_exists(&conn, "alpha", "beta"),
        "precondition: edge must exist"
    );
    drop(conn);

    fs::remove_file(env.cfg.memory_path.join(relative)).expect("remove memory file");
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert!(
        !relation_exists(&conn, "alpha", "beta"),
        "edge must disappear when its last backing file is removed"
    );
}

// T-4.3: RTM FR-23.5 — NOTE: FR-23 in RTM.md is HTTP Transport (collision);
// proximity weight cleanup requires PM RTM reconciliation.
#[test]
fn sync_preserves_remaining_source_file_weights_after_partial_delete() {
    let env = TestEnv::isolated();

    let remove_relative = "tests/remove-me.md";
    let keep_relative = "tests/keep-me.md";
    let remove_uri = write_memory_file(
        &env.cfg,
        remove_relative,
        // Both concepts in the same named section → proximity weight 1.0
        "# Remove\n\n## Identity\n[[alpha]] and [[beta]] share a named section.\n",
    );
    let keep_uri = write_memory_file(
        &env.cfg,
        keep_relative,
        // Both concepts in the same named section → proximity weight 1.0
        "# Keep\n\n## Identity\n[[alpha]] and [[beta]] stay together here.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    fs::remove_file(env.cfg.memory_path.join(remove_relative)).expect("remove one file");
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");
    assert!(
        source_files.get(&remove_uri).is_none(),
        "removed uri must disappear from source_files"
    );
    // Same-section pair → proximity weight 1.0 per file.
    // After deleting one of two contributors, the kept file's weight is preserved.
    assert_approx_eq(source_files[&keep_uri].as_f64().expect("kept weight"), 1.0);
    // co_occurrence_count = number of distinct contributing files = 1 (one file remains).
    assert_approx_eq(relation_count(&conn, "alpha", "beta").expect("count"), 1.0);
}

// T-4.3: RTM FR-23.5 — NOTE: FR-23 in RTM.md is HTTP Transport (collision);
// proximity weight cleanup requires PM RTM reconciliation.
#[test]
fn sync_preserves_cross_section_weight_after_partial_delete() {
    let env = TestEnv::isolated();

    let remove_relative = "tests/remove-cross.md";
    let keep_relative = "tests/keep-cross.md";
    let remove_uri = write_memory_file(
        &env.cfg,
        remove_relative,
        // Both concepts in the same named section → proximity weight 1.0
        "# Remove\n\n## Identity\n[[gamma]] and [[delta]] share a section.\n",
    );
    let keep_uri = write_memory_file(
        &env.cfg,
        keep_relative,
        // Concepts in different named sections → proximity weight 0.5
        "# Keep\n\n## Section A\n[[gamma]] appears here.\n\n## Section B\n[[delta]] appears here.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    fs::remove_file(env.cfg.memory_path.join(remove_relative)).expect("remove one file");
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "gamma", "delta");
    assert!(
        source_files.get(&remove_uri).is_none(),
        "removed uri must disappear from source_files"
    );
    // Cross-section pair → proximity weight 0.5 per file.
    // After deleting the same-section contributor (1.0), only the cross-section
    // file survives, so the kept weight must reflect the actual computed 0.5.
    assert_approx_eq(source_files[&keep_uri].as_f64().expect("kept weight"), 0.5);
    // co_occurrence_count = number of distinct contributing files = 1 (one file remains).
    assert_approx_eq(relation_count(&conn, "gamma", "delta").expect("count"), 1.0);
}
