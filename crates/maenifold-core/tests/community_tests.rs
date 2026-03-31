#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod community_build_context_degradation;
mod community_build_context_siblings;
mod community_coverage_topologies;
mod community_louvain_basics;
mod community_louvain_persistence;
mod community_runner;
mod community_sibling_scope;
mod community_sync;
