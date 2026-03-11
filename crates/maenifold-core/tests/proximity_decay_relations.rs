// T-12.0: RTM NFR-1
#[path = "proximity_decay/support.rs"]
mod support;

use support::{
    DecayEnvGuard, TestEnv, acquire_test_lock, assert_weight_eq, invoke_sync, open_db,
    relation_source_files, relation_weight_for_uri, write_memory_file,
};

// T-12.0: RTM NFR-1
#[test]
fn sync_same_h2_section_relations_get_full_proximity_weight() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-same-section.md",
        "# Same Section\n\n## Identity\nPair [[alpha]] with [[beta]] in the same section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let weight = relation_weight_for_uri(&conn, "alpha", "beta", &uri);
    assert_weight_eq(weight, 1.0);
}

// T-12.0: RTM NFR-1
#[test]
fn sync_cross_h2_section_relations_get_half_proximity_weight() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-cross-section.md",
        "# Cross Section\n\n## Identity\nFirst [[alpha]] appears here.\n\n## Sessions\nThen [[beta]] appears in a different section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let weight = relation_weight_for_uri(&conn, "alpha", "beta", &uri);
    assert_weight_eq(weight, 0.5);
}

// T-12.0: RTM NFR-1
#[test]
fn sync_without_h2_headings_stores_cross_section_pair_weights() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-no-h2.md",
        "# No H2\n\nInline concepts [[alpha]], [[beta]], and [[gamma]] share the implicit root section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_weight_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 0.5);
    assert_weight_eq(relation_weight_for_uri(&conn, "alpha", "gamma", &uri), 0.5);
}

// T-12.0: RTM NFR-1
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
