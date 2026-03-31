#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

// T-HOTFIX: RTM NFR-3
// Keeps vector/embedding integration coverage out of the default integration binary.

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

/// Policy guard: vector-heavy integration tests are opt-in.
/// Use an exact filter for a single test, `--list` for discovery, or
/// `MAENIFOLD_VECTOR_SUITE=1` to run the whole binary.
#[cfg(test)]
#[ctor::ctor]
fn guard_vector_suite_run() {
    let args: Vec<String> = std::env::args().collect();
    let has_positional_filter = args
        .iter()
        .skip(1)
        .any(|arg| !arg.is_empty() && !arg.starts_with('-'));
    let has_filter = has_positional_filter
        || args.iter().any(|arg| arg.contains("::"))
        || args
            .windows(2)
            .any(|window| window[0] == "--" && !window[1].is_empty());
    let has_skip = args.iter().any(|arg| arg == "--skip" || arg == "--exact");
    let is_listing = args.iter().any(|arg| arg == "--list");
    let opted_in = std::env::var("MAENIFOLD_VECTOR_SUITE").is_ok_and(|value| value == "1");

    if !has_filter && !has_skip && !is_listing && !opted_in {
        eprintln!("\n\x1b[1;33m╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║  Vector integration suite skipped by default.                 ║");
        eprintln!("║                                                                ║");
        eprintln!("║  Use a targeted test:                                          ║");
        eprintln!("║    cargo test -p maenifold-core --test vector_integration_tests║");
        eprintln!("║      -- <module>::<test_name>                                  ║");
        eprintln!("║                                                                ║");
        eprintln!("║  To run the full vector suite:                                 ║");
        eprintln!("║    MAENIFOLD_VECTOR_SUITE=1 cargo test -p maenifold-core       ║");
        eprintln!("║      --test vector_integration_tests                           ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝\x1b[0m\n");
        std::process::exit(0);
    }
}

mod sync_single_file_vec0_readiness;
mod vector_sync_diag_reporting;
mod vector_sync_diag_unavailable;
mod vector_tools_batch;
mod vector_tools_runtime;
mod vector_tools_semantic;
