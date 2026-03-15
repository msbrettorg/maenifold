// T-HOTFIX: RTM NFR-3
// Consolidates maenifold-core integration tests into a single binary.

// Single process-wide mutex for all tests that mutate the global Config state
// via `Config::override_root` / `Config::set_database_path`.  Each support
// module used to declare its own `TEST_MUTEX`, but because every test file
// includes support.rs as its *own* sub-module, those statics are distinct
// objects — tests from different modules could race.  All such support modules
// now re-export this one mutex, guaranteeing serialisation across the whole
// integration-test binary.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

/// Policy guard: running the full integration test suite is prohibited by default.
/// Individual test modules or targeted test patterns MUST be specified.
/// To explicitly opt in to a full suite run, set MAENIFOLD_FULL_SUITE=1.
///
/// This exists because the full suite takes 500+ seconds in serial mode and
/// AI agents have been observed running it repeatedly, wasting significant time.
#[cfg(test)]
#[ctor::ctor]
fn guard_full_suite_run() {
    let args: Vec<String> = std::env::args().collect();
    let has_filter = args.iter().any(|arg| arg.contains("::"))
        || args
            .windows(2)
            .any(|window| window[0] == "--" && !window[1].is_empty());
    let has_skip = args.iter().any(|a| a == "--skip" || a == "--exact");
    let opted_in = std::env::var("MAENIFOLD_FULL_SUITE").map_or(false, |v| v == "1");

    if !has_filter && !has_skip && !opted_in {
        eprintln!("\n\x1b[1;31m╔══════════════════════════════════════════════════════════════╗");
        eprintln!("║  POLICY VIOLATION: Full test suite runs are prohibited.      ║");
        eprintln!("║                                                              ║");
        eprintln!("║  Use targeted tests:                                         ║");
        eprintln!("║    cargo test -p maenifold-core --test integration_tests     ║");
        eprintln!("║      -- <module>::<test_name>                                ║");
        eprintln!("║                                                              ║");
        eprintln!("║  To override: MAENIFOLD_FULL_SUITE=1 cargo test ...          ║");
        eprintln!("╚══════════════════════════════════════════════════════════════╝\x1b[0m\n");
        std::process::exit(1);
    }
}

mod access_boosting_build_context;
mod access_boosting_reads;
mod access_boosting_search;
mod adopt_boundary;
mod adopt_tools_tests;
mod asset_manager_errors;
mod asset_manager_initialize;
mod asset_manager_update_detection;
mod asset_watcher_events;
mod asset_watcher_timing;
mod benchmark_boundary_controls;
mod benchmark_boundary_output;
mod community_build_context_degradation;
mod community_build_context_siblings;
mod community_coverage_topologies;
mod community_louvain_basics;
mod community_louvain_persistence;
mod community_runner;
mod community_sibling_scope;
mod community_sync;
mod concept_analysis_classification;
mod concept_analysis_core;
mod concept_analysis_counting;
mod concept_analysis_support;
mod concept_graph_context;
mod concept_graph_sections;
mod concept_sync_edge_lifecycle;
mod concept_sync_edge_stability;
mod config_decay_config;
mod config_decay_weights;
mod content_preview_tests;
mod contracts;
mod date_detection_path_tests;
mod date_detection_ts_tests;
mod date_detection_val_tests;
mod db_init_pragma_tests;
mod db_maintenance_tests;
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
mod degraded_diag_assets;
mod degraded_diag_graph;
mod dynamic_model_db;
mod dynamic_model_discovery_paths;
mod dynamic_model_loading;
mod dynamic_model_tokenizer;
mod frontmatter_repair;
mod frontmatter_sessions;
mod frontmatter_tool_flows;
mod frontmatter_write;
mod graph_analyzer_random_walk;
mod graph_analyzer_visualize_behavior;
mod graph_analyzer_visualize_validation;
mod graph_decay_access;
mod graph_decay_graph;
mod graph_decay_search;
#[cfg(feature = "http")]
mod http_transport_tests;
mod incremental_sync_boundary_guards;
mod incremental_sync_boundary_mutations;
mod incremental_sync_fts_optimize;
mod incremental_sync_tools_processing;
mod incremental_sync_tools_watcher;
mod incremental_sync_unification_guards;
mod incremental_sync_unification_lifecycle;
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
mod path_security_tests;
mod path_symlink_canonicalization;
mod proximity_decay_relations;
mod proximity_decay_search;
mod proximity_weight_cleanup;
mod proximity_weight_communities;
mod proximity_weight_decay;
mod proximity_weight_sections;
mod recent_activity_boundary_queries;
mod recent_activity_boundary_rendering;
mod recent_activity_pipeline_collect;
mod recent_activity_pipeline_format;
mod recent_activity_pipeline_helpers;
mod recent_activity_pipeline_tool;
mod recent_activity_tool_behavior;
mod repair_concepts_batch;
mod repair_concepts_communities;
mod repair_concepts_creation;
mod repair_concepts_replacement;
mod safe_json_tests;
mod search_graph_boundary;
mod search_status_parity_tests;
mod search_tools_tests;
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
mod similar_concepts_plateau_tests;
mod sync_leader_gating;
mod sync_leader_lifecycle;
mod sync_mtime_opt_guards;
mod sync_mtime_skip;
mod sync_perf_baseline;
mod sync_perf_coordination;
mod sync_perf_graph;
mod sync_perf_indexing;
mod sync_perf_support;
mod sync_single_file_vec0_readiness;
mod sync_unification_batch;
mod sync_unification_single_file;
mod sync_wikilink_filter;
mod tool_registry_tests;
mod update_assets_boundary;
mod vector_sync_boundary;
mod vector_sync_diag_reporting;
mod vector_sync_diag_unavailable;
mod vector_tools_batch;
mod vector_tools_runtime;
mod vector_tools_semantic;
mod wikilink_filter_integration_tests;
mod wikilink_filter_unit_tests;
mod wikilink_regex_tests;
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
