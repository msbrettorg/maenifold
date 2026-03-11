// T-15.21: RTM FR-15 — Tests for FTS optimize scheduling in incremental sync.
// Verifies that process_incremental_batch triggers FTS optimize (via run_light_maintenance)
// only after accumulated file changes reach the Config.incremental_optimize_every threshold,
// matching C# IncrementalSyncTools.ScheduleMaintenanceIfNeeded() batching strategy.

#[path = "incremental_sync_tools/support.rs"]
mod support;

use support::{
    ConfigOverrideReset, TEST_MUTEX, changes_since_last_optimize, context, create_test_file,
    isolated_config, process_pending_paths_for_test, reset_optimize_counter,
};

// T-15.21: RTM FR-15
// After a batch with enough files to reach the optimize threshold, the counter
// resets to zero — proving schedule_maintenance_if_needed fired and ran
// run_light_maintenance (FTS optimize + passive WAL checkpoint).
#[test]
fn batch_sync_triggers_fts_optimize_after_reaching_threshold() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    // Set threshold low for test: optimize after 3 accumulated changes.
    unsafe { std::env::set_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", "3") };
    reset_optimize_counter();

    // Create 3 files (exactly at threshold).
    let mut paths = Vec::new();
    for i in 0..3 {
        let content = format!("# Note {i}\n\n[[fts-optimize-concept-{i}]]\n");
        let (path, _uri) = create_test_file(&cfg, "tests/fts-optimize", &content);
        paths.push(path);
    }

    process_pending_paths_for_test(&ctx, &paths).expect("batch sync");

    // Counter should be 0 — threshold was reached and optimize fired.
    assert_eq!(
        changes_since_last_optimize(),
        0,
        "optimize counter should reset to 0 after reaching threshold"
    );

    unsafe { std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY") };
}

// T-15.21: RTM FR-15
// A single file sync below the threshold should NOT trigger FTS optimize.
// The counter should reflect the accumulated (but not yet optimized) changes.
#[test]
fn single_file_below_threshold_does_not_trigger_optimize() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    // Set threshold to 10 — well above a single file.
    unsafe { std::env::set_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", "10") };
    reset_optimize_counter();

    let content = "# Single Note\n\n[[fts-single-concept]]\n";
    let (path, _uri) = create_test_file(&cfg, "tests/fts-optimize", content);

    process_pending_paths_for_test(&ctx, &[path]).expect("single file sync");

    // Counter should be 1 — below threshold, no optimize.
    assert_eq!(
        changes_since_last_optimize(),
        1,
        "optimize counter should accumulate without resetting below threshold"
    );

    unsafe { std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY") };
}

// T-15.21: RTM FR-15
// Multiple small batches that individually stay below threshold should
// accumulate and eventually trigger optimize when the total crosses it.
#[test]
fn accumulated_small_batches_trigger_optimize_at_threshold() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    // Threshold of 4: two batches of 2 files each should trigger.
    unsafe { std::env::set_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", "4") };
    reset_optimize_counter();

    // First batch: 2 files → counter = 2 (below threshold).
    let mut batch1 = Vec::new();
    for i in 0..2 {
        let content = format!("# Batch1 Note {i}\n\n[[accum-concept-b1-{i}]]\n");
        let (path, _uri) = create_test_file(&cfg, "tests/fts-optimize-accum", &content);
        batch1.push(path);
    }
    process_pending_paths_for_test(&ctx, &batch1).expect("batch 1");
    assert_eq!(
        changes_since_last_optimize(),
        2,
        "counter should be 2 after first batch"
    );

    // Second batch: 2 more files → accumulated = 4, reaches threshold.
    let mut batch2 = Vec::new();
    for i in 0..2 {
        let content = format!("# Batch2 Note {i}\n\n[[accum-concept-b2-{i}]]\n");
        let (path, _uri) = create_test_file(&cfg, "tests/fts-optimize-accum", &content);
        batch2.push(path);
    }
    process_pending_paths_for_test(&ctx, &batch2).expect("batch 2");

    // Counter should be 0 — accumulated total (4) hit threshold (4), optimize fired.
    assert_eq!(
        changes_since_last_optimize(),
        0,
        "optimize counter should reset after accumulated batches reach threshold"
    );

    unsafe { std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY") };
}

// T-15.21: RTM FR-15
// Empty batch (no valid files) should not increment the counter.
#[test]
fn empty_batch_does_not_increment_optimize_counter() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    unsafe { std::env::set_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", "5") };
    reset_optimize_counter();

    // Process an empty batch.
    process_pending_paths_for_test(&ctx, &[]).expect("empty batch");

    assert_eq!(
        changes_since_last_optimize(),
        0,
        "empty batch should not increment optimize counter"
    );

    unsafe { std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY") };
}
