#[path = "benchmark_boundary/support.rs"]
mod support;

use serde_json::json;
use support::{
    TEST_MUTEX, invoke_benchmark, invoke_benchmark_result, isolated_env, open_db,
    seed_file_content_bulk, seed_graph_data,
};

// T-12.7: RTM NFR-1 - Split benchmark boundary control-path assertions into a focused file.

// RTM-032 / TC-5.2
#[test]
fn benchmark_iterations_parameter_reflected_in_output() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "iterations": 1 }));
    assert!(
        output.contains("Iterations: 1"),
        "expected 'Iterations: 1' in output: {output}"
    );

    let output3 = invoke_benchmark(json!({ "iterations": 3 }));
    assert!(
        output3.contains("Iterations: 3"),
        "expected 'Iterations: 3' in output: {output3}"
    );
}

// RTM-033 / TC-5.3
#[test]
fn benchmark_max_test_files_reflected_in_output() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "iterations": 1, "maxTestFiles": 42 }));
    assert!(
        output.contains("Max Test Files: 42"),
        "expected 'Max Test Files: 42' in output: {output}"
    );

    let output2 = invoke_benchmark(json!({ "iterations": 1, "maxTestFiles": 999 }));
    assert!(
        output2.contains("Max Test Files: 999"),
        "expected 'Max Test Files: 999' in output: {output2}"
    );
}

// RTM-034 / TC-5.4
#[test]
fn benchmark_exclude_deep_traversal_skips_complex_section_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);

    seed_file_content_bulk(&conn, 110);
    seed_graph_data(&conn, 5, 10);

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

    let output_included = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": true
    }));
    assert!(
        !output_included.contains("Skipped: includeDeepTraversal=false"),
        "should not contain skip message when deep traversal is enabled: {output_included}"
    );
}

// RTM-032 / TC-5.2
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
