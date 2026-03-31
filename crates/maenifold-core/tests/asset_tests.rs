#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod adopt_boundary;
mod adopt_tools_tests;
mod asset_manager_errors;
mod asset_manager_initialize;
mod asset_manager_update_detection;
mod asset_watcher_events;
mod asset_watcher_timing;
mod update_assets_boundary;
