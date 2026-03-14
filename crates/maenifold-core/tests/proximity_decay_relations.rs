// T-12.0: RTM NFR-2
#[path = "proximity_decay/support.rs"]
mod support;

use support::{
    DecayEnvGuard, TestEnv, acquire_test_lock, assert_weight_eq, invoke_sync, open_db,
    relation_source_files, write_memory_file,
};

// T-12.0: RTM NFR-2
#[test]
fn sync_stores_source_files_as_json_weight_dictionary() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    let same_uri = write_memory_file(
        &env.cfg,
        "projects/proximity-source-files-same.md",
        "# Same\n\n## Identity\nThis file links [[alpha]] and [[beta]].\n",
    );
    let cross_uri = write_memory_file(
        &env.cfg,
        "projects/proximity-source-files-cross.md",
        "# Cross\n\n## First\nThis file mentions [[alpha]].\n\n## Second\nThis file mentions [[beta]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");
    let source_files = source_files
        .as_object()
        .expect("source_files should deserialize to a JSON object");

    assert_eq!(source_files.len(), 2, "unexpected source_files payload");
    assert_weight_eq(
        source_files[&same_uri]
            .as_f64()
            .expect("same-section weight should be numeric"),
        1.0,
    );
    assert_weight_eq(
        source_files[&cross_uri]
            .as_f64()
            .expect("cross-section weight should be numeric"),
        0.5,
    );
}
