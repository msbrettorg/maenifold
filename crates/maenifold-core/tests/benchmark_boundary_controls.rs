#[path = "benchmark_boundary/support.rs"]
mod support;

use maenifold_core::json::OutputFormat;
use maenifold_core::ToolRegistry;
use serde_json::json;
use support::{
    TEST_MUTEX, invoke_benchmark, invoke_benchmark_result, isolated_env, write_memory_file,
};

// T-12.7: RTM NFR-1 - Split benchmark boundary control-path assertions into a focused file.

// T-12.7: RTM-032 / TC-5.2, RTM-033 / TC-5.3 — header reflects passed parameters
// MERGE of former benchmark_iterations_parameter_reflected_in_output
// and benchmark_max_test_files_reflected_in_output (same behavioral contract).
#[test]
fn benchmark_header_parameters_reflected_in_output() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    // --- iterations parameter echo ---
    let output1 = invoke_benchmark(json!({ "iterations": 1 }));
    assert!(
        output1.contains("Iterations: 1"),
        "expected 'Iterations: 1' in output: {output1}"
    );

    let output3 = invoke_benchmark(json!({ "iterations": 3 }));
    assert!(
        output3.contains("Iterations: 3"),
        "expected 'Iterations: 3' in output: {output3}"
    );

    // --- maxTestFiles parameter echo ---
    let output_42 = invoke_benchmark(json!({ "iterations": 1, "maxTestFiles": 42 }));
    assert!(
        output_42.contains("Max Test Files: 42"),
        "expected 'Max Test Files: 42' in output: {output_42}"
    );

    let output_999 = invoke_benchmark(json!({ "iterations": 1, "maxTestFiles": 999 }));
    assert!(
        output_999.contains("Max Test Files: 999"),
        "expected 'Max Test Files: 999' in output: {output_999}"
    );
}

// T-12.7: RTM-034 / TC-5.4 — includeDeepTraversal flag gates execution
#[test]
fn benchmark_deep_traversal_flag_controls_execution() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Seed real .md files with WikiLinks establishing hub-spoke graph topology.
    // Real files survive the benchmark's internal sync (unlike DB-only seeds).
    // 5 hubs × 10 files = 50 hub files + 60 general = 110 total (≥100 for search).
    for h in 0..5 {
        for e in 0..10 {
            let content = format!(
                "# Benchmark Hub {h} File {e}\n\n\
                 [[hub-concept-{h}]] connects to [[leaf-{h}-{e}]].\n"
            );
            write_memory_file(&env.cfg, &format!("hub{h}/file-{h}-{e}.md"), &content);
        }
    }
    for i in 0..60 {
        let content = format!(
            "# General Benchmark File {i}\n\n\
             Content about [[topic-{i}]] for search corpus padding.\n"
        );
        write_memory_file(&env.cfg, &format!("general/file-{i}.md"), &content);
    }

    // Pre-sync populates DB from real files so entries survive internal sync.
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("pre-sync");

    // --- true path: traversal actually executes and emits results ---
    let output_included = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": true
    }));
    assert!(
        !output_included.contains("Skipped: includeDeepTraversal=false"),
        "should not contain skip message when deep traversal is enabled: {output_included}"
    );
    assert!(
        output_included.contains("Hub concepts:"),
        "expected deep traversal to execute and report hub concepts: {output_included}"
    );
    assert!(
        output_included.contains("Average time:"),
        "expected traversal timing output: {output_included}"
    );

    // --- false path: section header present but traversal skipped ---
    let output = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": false
    }));
    assert!(
        output.contains("Complex Traversal Bottleneck Analysis"),
        "expected section header present: {output}"
    );
    assert!(
        output.contains("Skipped: includeDeepTraversal=false"),
        "expected skip message: {output}"
    );
}

// T-12.7: RTM-032 / TC-5.2 — lower bound
#[test]
fn benchmark_iterations_zero_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_benchmark_result(json!({ "iterations": 0 }));
    assert!(result.is_err(), "iterations=0 should return an error");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("iterations must be greater than zero"),
        "unexpected error message: {err_msg}"
    );
}

// T-12.7: RTM-032 / TC-5.2 — upper bound
#[test]
fn benchmark_iterations_above_maximum_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_benchmark_result(json!({ "iterations": 101 }));
    assert!(result.is_err(), "iterations=101 should return an error");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("iterations must be between 1 and 100"),
        "unexpected error message: {err_msg}"
    );
}
