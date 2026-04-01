#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod config_decay_config;
mod config_decay_weights;
mod decay_benchmark_search;
mod decay_benchmark_weights;
mod decay_calculator_active_refined_fallbacks;
mod decay_calculator_path_grace;
mod decay_calculator_power_law;
mod decay_calculator_validated_invalidated;
mod decay_function_boundaries;
mod decay_function_exponential;
mod decay_function_grace_periods;
mod decay_function_power_law;
mod graph_decay_access;
mod graph_decay_graph;
mod graph_decay_search;
