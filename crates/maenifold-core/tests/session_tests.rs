#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod session_assumption_crud;
mod session_assumption_misc;
mod session_assumption_validate_append;
mod session_assumption_validate_update;
mod session_cleanup_sweep;
mod session_cleanup_ttl;
mod session_sequential_errors;
mod session_sequential_flow;
mod session_sequential_lifecycle;
mod session_sequential_status;
mod session_workflow_boundary;
