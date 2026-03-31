#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod incremental_sync_boundary_guards;
mod incremental_sync_boundary_mutations;
mod incremental_sync_fts_optimize;
mod incremental_sync_tools_processing;
mod incremental_sync_tools_watcher;
mod incremental_sync_unification_guards;
mod incremental_sync_unification_lifecycle;
mod sync_leader_gating;
mod sync_leader_lifecycle;
mod sync_mtime_opt_guards;
mod sync_mtime_skip;
mod sync_perf_baseline;
mod sync_perf_coordination;
mod sync_perf_graph;
mod sync_perf_indexing;
mod sync_perf_support;
mod sync_unification_batch;
mod sync_unification_single_file;
mod sync_wikilink_filter;
