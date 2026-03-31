#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod graph_analyzer_random_walk;
mod graph_analyzer_visualize_behavior;
mod graph_analyzer_visualize_validation;
mod proximity_decay_relations;
mod proximity_decay_search;
mod proximity_weight_cleanup;
mod proximity_weight_communities;
mod proximity_weight_decay;
mod proximity_weight_sections;
