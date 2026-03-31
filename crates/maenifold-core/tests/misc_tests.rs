#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod access_boosting_build_context;
mod access_boosting_reads;
mod access_boosting_search;
mod benchmark_boundary_controls;
mod benchmark_boundary_output;
mod content_preview_tests;
mod contracts;
mod date_detection_path_tests;
mod date_detection_ts_tests;
mod date_detection_val_tests;
mod db_init_pragma_tests;
mod db_maintenance_tests;
mod degraded_diag_assets;
mod degraded_diag_graph;
mod dynamic_model_db;
mod dynamic_model_discovery_paths;
mod dynamic_model_loading;
mod dynamic_model_tokenizer;
mod http_transport_tests;
mod path_security_tests;
mod path_symlink_canonicalization;
mod safe_json_tests;
mod tool_registry_tests;
