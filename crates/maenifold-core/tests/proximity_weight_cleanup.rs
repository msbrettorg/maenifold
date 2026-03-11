// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use std::fs;
use support::{
    TestEnv, acquire_test_lock, assert_approx_eq, invoke_sync, open_db, relation_count,
    relation_exists, relation_source_files, write_memory_file,
};

// T-4.3: RTM FR-23.5
#[test]
fn sync_removes_edge_when_last_source_file_is_deleted() {
    let _guard = acquire_test_lock();
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

// T-4.3: RTM FR-23.5
#[test]
fn sync_preserves_remaining_source_file_weights_after_partial_delete() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();

    let remove_relative = "tests/remove-me.md";
    let keep_relative = "tests/keep-me.md";
    let remove_uri = write_memory_file(
        &env.cfg,
        remove_relative,
        "# Remove\n\n## Identity\n[[alpha]] and [[beta]] share a named section.\n",
    );
    let keep_uri = write_memory_file(
        &env.cfg,
        keep_relative,
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
        source_files.is_object(),
        "source_files must remain a json object"
    );
    assert!(
        source_files.get(&remove_uri).is_none(),
        "removed uri must disappear"
    );
    assert_approx_eq(source_files[&keep_uri].as_f64().expect("kept weight"), 1.0);
    assert_approx_eq(relation_count(&conn, "alpha", "beta").expect("count"), 1.0);
}
