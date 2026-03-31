#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod workflow_append;
mod workflow_completion;
mod workflow_lifecycle;
mod workflow_progress;
mod workflow_queue;
mod workflow_supervisor_cleanup_core;
mod workflow_supervisor_cleanup_guards;
mod workflow_supervisor_cleanup_thresholds;
mod workflow_supervisor_lifecycle;
mod workflow_supervisor_recovery;
mod workflow_supervisor_registration;
mod workflow_supervisor_sync_cleanup;
mod workflow_view;
