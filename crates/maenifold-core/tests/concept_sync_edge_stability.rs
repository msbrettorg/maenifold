#[path = "concept_sync_edge/support.rs"]
mod support;

use std::collections::BTreeMap;
use support::*;

// T-12.7: RTM FR-14 - split concept-sync edge coverage by stability-oriented regressions.
// This file owns multi-edge rowid preservation across section types (same-section + cross-section).
// See also: sync_perf_graph.rs::build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct
//   — that test owns "no duplicate rows + correct co_occurrence_count" for a single same-section pair.

// T-5.7: RTM FR-14.3
// Exercises the `should_write = false` guard in graph_state.rs for all three edge pairs.
// When wikilink structure is unchanged (prose-only edit), no graph rows should be rewritten
// and both rowids and weights must be preserved.
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

    // Capture edge data before resync to verify both identity (rowid) and data (weights).
    let edges_before = BTreeMap::from([
        (
            ("alpha".to_string(), "beta".to_string()),
            query_edge(&cfg, "alpha", "beta").expect("alpha-beta edge before"),
        ),
        (
            ("alpha".to_string(), "gamma".to_string()),
            query_edge(&cfg, "alpha", "gamma").expect("alpha-gamma edge before"),
        ),
        (
            ("beta".to_string(), "gamma".to_string()),
            query_edge(&cfg, "beta", "gamma").expect("beta-gamma edge before"),
        ),
    ]);

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]] [[beta]]\n\nExtra prose without new wikilinks.\n\n## Section Two\n\n[[gamma]]\n",
    );

    let resync = sync_one(&cfg, relative_path);
    // T-5.7: RTM FR-14.3 — confirm resync processed the correct file and evaluated all
    // 3 edge pairs. Note: relation_count counts pairs *evaluated*, not pairs *written*.
    // The should_write=false guard prevents DB writes for unchanged edges; the rowid +
    // weight assertions below verify no graph rows were actually rewritten.
    assert!(
        resync.contains("memory://concept-sync/file-a")
            && resync.contains("3 affected relations rebuilt"),
        "resync must evaluate all 3 edge pairs for the correct file: {resync}"
    );

    // Verify rowids are unchanged — the should_write=false path preserves rowids by doing nothing.
    for ((left, right), rowid_before) in &rowids_before {
        assert_eq!(
            query_edge_rowid(&cfg, left, right),
            Some(*rowid_before),
            "re-syncing unchanged concept sections must not rewrite {left}-{right}"
        );
    }

    // Verify edge data (weights) are preserved alongside rowids — catches bugs that
    // reset weights to 0.0 while preserving rowids.
    for ((left, right), (count_before, files_before)) in &edges_before {
        let (count_after, files_after) = query_edge(&cfg, left, right)
            .unwrap_or_else(|| panic!("{left}-{right} edge missing after resync"));
        assert_eq!(
            count_after, *count_before,
            "{left}-{right} co_occurrence_count must be unchanged after prose-only resync"
        );
        assert_eq!(
            files_after, *files_before,
            "{left}-{right} source_files must be unchanged after prose-only resync"
        );
    }
}

// T-5.7: RTM FR-14.3
// Exercises two distinct code paths in graph_state.rs:
//   - should_write=false for alpha-gamma (co_occurrence_count + source_files unchanged).
//   - ON CONFLICT DO UPDATE for alpha-beta and beta-gamma (source_files changed, rowid
//     preserved by SQLite upsert mechanics, not by skipping the write).
#[test]
fn section_change_rewrites_only_affected_edges() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-b.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(&cfg, relative_path, initial_sectioned_content());
    sync_one(&cfg, relative_path);

    // Capture rowids for all three edges before the section change.
    // alpha-gamma: will exercise the should_write=false path (no data change).
    // alpha-beta, beta-gamma: will exercise the ON CONFLICT DO UPDATE path
    //   (data changes, rowid preserved by upsert — not by skipping the write).
    let alpha_gamma_rowid_before =
        query_edge_rowid(&cfg, "alpha", "gamma").expect("alpha-gamma rowid");
    let alpha_beta_rowid_before =
        query_edge_rowid(&cfg, "alpha", "beta").expect("alpha-beta rowid");
    let beta_gamma_rowid_before =
        query_edge_rowid(&cfg, "beta", "gamma").expect("beta-gamma rowid");

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]]\n\n## Section Two\n\n[[beta]] [[gamma]]\n",
    );

    let output = sync_one(&cfg, relative_path);
    // T-5.7: RTM FR-14.3 — section change evaluates all 3 relation pairs.
    // Note: relation_count counts pairs *evaluated*, not pairs *written*. The
    // should_write guard skips the DB write for alpha-gamma (unchanged) but still
    // counts it. Weight + rowid assertions below verify actual write behavior.
    assert!(
        output.contains("3 affected relations rebuilt"),
        "section-change must evaluate all 3 edge pairs: {output}"
    );

    // alpha-gamma: rowid preserved because co_occurrence_count and source_files are
    // identical — the should_write guard returns false, no DB write occurs.
    assert_eq!(
        query_edge_rowid(&cfg, "alpha", "gamma"),
        Some(alpha_gamma_rowid_before),
        "alpha-gamma rowid must be preserved via should_write=false (no data change)"
    );

    // alpha-gamma weight must also remain 0.5 — verifies the full row is intact,
    // not just the identifier.
    let alpha_gamma =
        query_edge(&cfg, "alpha", "gamma").expect("alpha-gamma edge after section change");
    assert_eq!(
        alpha_gamma.1.get(&uri).copied(),
        Some(0.5),
        "alpha-gamma weight must remain 0.5 — section move did not affect this pair"
    );

    // alpha-beta, beta-gamma: rowids preserved via ON CONFLICT DO UPDATE (upsert).
    // If this were accidentally replaced with INSERT OR REPLACE, it would
    // delete+reinsert and assign a new rowid — these assertions catch that regression.
    assert_eq!(
        query_edge_rowid(&cfg, "alpha", "beta"),
        Some(alpha_beta_rowid_before),
        "alpha-beta rowid must be preserved via ON CONFLICT DO UPDATE even when weight changes"
    );
    assert_eq!(
        query_edge_rowid(&cfg, "beta", "gamma"),
        Some(beta_gamma_rowid_before),
        "beta-gamma rowid must be preserved via ON CONFLICT DO UPDATE even when weight changes"
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
