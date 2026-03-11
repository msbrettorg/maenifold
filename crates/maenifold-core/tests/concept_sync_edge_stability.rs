#[path = "concept_sync_edge/support.rs"]
mod support;

use std::collections::BTreeMap;
use support::*;

// T-12.7: RTM FR-14 - split concept-sync edge coverage by stability-oriented regressions.

// T-5.7: RTM FR-14.3
#[test]
fn identical_resync_preserves_existing_edge_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-a.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(&cfg, relative_path, initial_sectioned_content());

    let first_sync = sync_one(&cfg, relative_path);
    assert!(
        first_sync.contains("Synced memory://concept-sync/file-a"),
        "initial sync should succeed: {first_sync}"
    );

    let rowids_before = BTreeMap::from([
        (
            ("alpha".to_string(), "beta".to_string()),
            query_edge_rowid(&cfg, "alpha", "beta").expect("alpha-beta rowid"),
        ),
        (
            ("alpha".to_string(), "gamma".to_string()),
            query_edge_rowid(&cfg, "alpha", "gamma").expect("alpha-gamma rowid"),
        ),
        (
            ("beta".to_string(), "gamma".to_string()),
            query_edge_rowid(&cfg, "beta", "gamma").expect("beta-gamma rowid"),
        ),
    ]);

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]] [[beta]]\n\nExtra prose without new wikilinks.\n\n## Section Two\n\n[[gamma]]\n",
    );

    let resync = sync_one(&cfg, relative_path);
    assert!(
        resync.contains("concept mentions updated"),
        "resync should process file content changes: {resync}"
    );

    for ((left, right), rowid_before) in rowids_before {
        assert_eq!(
            query_edge_rowid(&cfg, &left, &right),
            Some(rowid_before),
            "re-syncing unchanged concept sections must not rewrite {left}-{right}"
        );
    }
}

// T-5.7: RTM FR-14.3
#[test]
fn section_change_rewrites_only_affected_edges() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-b.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(&cfg, relative_path, initial_sectioned_content());
    sync_one(&cfg, relative_path);

    let alpha_gamma_rowid_before =
        query_edge_rowid(&cfg, "alpha", "gamma").expect("alpha-gamma rowid");

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]]\n\n## Section Two\n\n[[beta]] [[gamma]]\n",
    );

    let output = sync_one(&cfg, relative_path);
    assert!(
        output.contains("affected relations rebuilt"),
        "section-change sync should complete: {output}"
    );

    assert_eq!(
        query_edge_rowid(&cfg, "alpha", "gamma"),
        Some(alpha_gamma_rowid_before),
        "alpha-gamma should keep its rowid because the edge weight stayed unchanged"
    );

    let alpha_beta = query_edge(&cfg, "alpha", "beta").expect("alpha-beta edge");
    assert_eq!(
        alpha_beta.1.get(&uri).copied(),
        Some(0.5),
        "alpha-beta weight must drop to 0.5 after beta moves to section two"
    );

    let beta_gamma = query_edge(&cfg, "beta", "gamma").expect("beta-gamma edge");
    assert_eq!(
        beta_gamma.1.get(&uri).copied(),
        Some(1.0),
        "beta-gamma weight must rise to 1.0 once both concepts share section two"
    );
}
