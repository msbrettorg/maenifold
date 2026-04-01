#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod search_graph_boundary;
mod search_status_parity_tests;
mod search_tools_tests;
mod wikilink_filter_integration_tests;
mod wikilink_filter_unit_tests;
mod wikilink_regex_tests;
