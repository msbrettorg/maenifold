#[allow(clippy::duplicate_mod)]
#[path = "concept_sync_edge/support.rs"]
mod support;

use support::*;

// T-12.7: RTM FR-14 - split concept-sync edge coverage by edge lifecycle regressions.

// T-5.7: RTM FR-14.3
#[test]
fn concept_removed_cleans_stale_edge_contributions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-c.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]] [[gamma]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge")
            .1
            .contains_key(&uri)
    );
    assert!(
        query_edge(&cfg, "alpha", "gamma")
            .expect("alpha-gamma edge")
            .1
            .contains_key(&uri)
    );
    assert!(
        query_edge(&cfg, "beta", "gamma")
            .expect("beta-gamma edge")
            .1
            .contains_key(&uri)
    );

    // Force re-sync: the mtime cache skips unchanged files. Backdating last_indexed
    // makes the engine treat this file as stale on the next sync_one() call.
    // If the cache strategy changes to content-hash, update this call site.
    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge after removal")
            .1
            .contains_key(&uri),
        "alpha-beta should keep the file contribution after gamma is removed"
    );
    // T-5.7: RTM FR-14.3 — sole-contributor edges must be fully deleted, not just
    // contribution-pruned. In this isolated single-file config, removing gamma leaves
    // zero contributors for alpha-gamma and beta-gamma; zombie rows are a real bug.
    assert!(
        query_edge(&cfg, "alpha", "gamma").is_none(),
        "alpha-gamma row must be deleted when its only contributor removes the concept"
    );
    assert!(
        query_edge(&cfg, "beta", "gamma").is_none(),
        "beta-gamma row must be deleted when its only contributor removes the concept"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn new_concept_added_creates_new_edge_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-d.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(query_edge(&cfg, "alpha", "beta").is_some());
    assert!(query_edge(&cfg, "alpha", "gamma").is_none());
    assert!(query_edge(&cfg, "beta", "gamma").is_none());

    // Force re-sync: the mtime cache skips unchanged files. Backdating last_indexed
    // makes the engine treat this file as stale on the next sync_one() call.
    // If the cache strategy changes to content-hash, update this call site.
    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]] [[gamma]]\n",
    );
    sync_one(&cfg, relative_path);

    let alpha_gamma = query_edge(&cfg, "alpha", "gamma").expect("alpha-gamma edge");
    let beta_gamma = query_edge(&cfg, "beta", "gamma").expect("beta-gamma edge");
    assert!(
        alpha_gamma.1.contains_key(&uri),
        "alpha-gamma should reference the contributing file once gamma is added"
    );
    assert!(
        beta_gamma.1.contains_key(&uri),
        "beta-gamma should reference the contributing file once gamma is added"
    );
    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge")
            .1
            .contains_key(&uri),
        "existing alpha-beta edge should be preserved"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn last_contributor_removed_deletes_edge_row() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-e.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);
    assert!(query_edge(&cfg, "alpha", "beta").is_some());

    // Force re-sync: the mtime cache skips unchanged files. Backdating last_indexed
    // makes the engine treat this file as stale on the next sync_one() call.
    // If the cache strategy changes to content-hash, update this call site.
    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta").is_none(),
        "alpha-beta row should be deleted when no files contribute the pair anymore"
    );
}
