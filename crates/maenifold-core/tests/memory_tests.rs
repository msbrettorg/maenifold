#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod memory_boundary_paths;
mod memory_lifecycle_validation;
mod memory_path_nested;
mod memory_path_platform;
mod memory_path_security;
mod memory_search_diag_tests;
mod memory_status_parity;
mod memory_tools_edit_ops;
mod memory_tools_editing_parity;
mod memory_tools_listing;
mod memory_tools_move_read_write;
mod memory_tools_replace_sections;
mod memory_tools_sanitization;
mod memory_tools_wikilink_replacements;
