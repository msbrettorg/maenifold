#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod concept_analysis_classification;
mod concept_analysis_core;
mod concept_analysis_counting;
mod concept_analysis_support;
mod concept_graph_context;
mod concept_graph_sections;
mod concept_sync_edge_lifecycle;
mod concept_sync_edge_stability;
mod similar_concepts_plateau_tests;
