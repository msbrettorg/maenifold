#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod markdown_adversarial_code;
mod markdown_adversarial_structure;
mod markdown_boundary_io;
mod markdown_boundary_listing;
mod markdown_boundary_redteam;
mod markdown_boundary_validation;
mod markdown_rw_extract_tests;
mod markdown_rw_normalize_tests;
mod markdown_rw_read_tests;
mod markdown_rw_sync_tests;
mod markdown_rw_write_tests;
