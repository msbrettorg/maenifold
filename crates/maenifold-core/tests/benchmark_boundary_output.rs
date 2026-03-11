#[path = "benchmark_boundary/support.rs"]
mod support;

use serde_json::json;
use support::{
    TEST_MUTEX, invoke_benchmark, isolated_env, open_db, seed_file_content_bulk, seed_graph_data,
    write_memory_file,
};

// T-12.7: RTM NFR-1 - Split benchmark boundary output assertions into a focused file.

// RTM-031 / RTM-037 / TC-5.1
#[test]
fn benchmark_empty_db_contains_section_headers_or_insufficient_data() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "iterations": 1 }));

    assert!(
        output.contains("MA CORE PERFORMANCE BENCHMARK SUITE"),
        "missing suite header: {output}"
    );
    assert!(
        output.contains("Insufficient test data"),
        "expected 'Insufficient test data' message for empty db: {output}"
    );
    assert!(
        output.contains("GRPH-009: CTE vs N+1 Pattern Benchmark"),
        "missing graph benchmark header: {output}"
    );
    assert!(
        output.contains("System Health Report"),
        "missing system health section on early return: {output}"
    );
}

// RTM-035 / TC-5.5
#[test]
fn benchmark_with_data_contains_timing_statistics() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);

    seed_file_content_bulk(&conn, 110);
    seed_graph_data(&conn, 3, 8);
    for i in 0..5 {
        write_memory_file(
            &env.cfg,
            &format!("benchmark-timing/note-{i}.md"),
            &format!(
                "---\ntitle: Timing Note {i}\n---\n\n# Timing Note {i}\n\n\
                 Testing [[benchmark]] and [[performance]] with [[graph traversal]].\n"
            ),
        );
    }

    let output = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": true
    }));

    assert!(
        output.contains("Search Performance Benchmark"),
        "missing search benchmark section: {output}"
    );
    assert!(
        output.contains("Hybrid Average:"),
        "missing Hybrid Average: {output}"
    );
    assert!(
        output.contains("Hybrid Min/Max/Median:"),
        "missing Hybrid Min/Max/Median: {output}"
    );
    assert!(
        output.contains("Semantic Min/Max/Median:"),
        "missing Semantic Min/Max/Median: {output}"
    );
    assert!(
        output.contains("FullText Min/Max/Median:"),
        "missing FullText Min/Max/Median: {output}"
    );
    assert!(
        output.contains("Sync Performance Benchmark"),
        "missing sync benchmark section: {output}"
    );
    assert!(
        output.contains("Sync Min/Max/Median:"),
        "missing Sync Min/Max/Median: {output}"
    );
    assert!(
        output.contains("ms"),
        "expected 'ms' unit in timing output: {output}"
    );
}

// RTM-036 / TC-5.6
#[test]
fn benchmark_learn_true_returns_help_documentation() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "learn": true }));

    assert!(
        output.contains("RunFullBenchmark"),
        "expected tool name in help output: {output}"
    );
    assert!(
        output.contains("iterations") || output.contains("Iterations"),
        "expected parameter docs: {output}"
    );
    assert!(
        output.contains("includeDeepTraversal") || output.contains("deep traversal"),
        "expected deep traversal parameter docs: {output}"
    );

    let outside_code_blocks: String = output.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("Started: 20"),
        "learn=true should not produce live benchmark output: {output}"
    );
}
