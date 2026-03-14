#[path = "benchmark_boundary/support.rs"]
mod support;

use regex::Regex;
use serde_json::json;
use support::{
    TEST_MUTEX, invoke_benchmark, isolated_env, open_db, seed_file_content_bulk, seed_graph_data,
    write_memory_file,
};

// T-12.7: RTM NFR-1 - Split benchmark boundary output assertions into a focused file.

/// The benchmark tool requires at least 100 indexed files to produce timing output.
/// See `render_insufficient_search_data_report` in `benchmark/render.rs`.
const BENCHMARK_MIN_FILES: usize = 100;

// RTM-031 / RTM-037 / TC-5.1
// T-12.7: RTM FR-5 — empty DB must surface insufficient-data AND section headers,
// and must NOT emit timing statistics.
#[test]
fn benchmark_empty_db_shows_insufficient_data_and_section_headers() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    // iterations=1: minimum iteration count to exercise the empty-DB early-return path
    let output = invoke_benchmark(json!({ "iterations": 1 }));

    assert!(
        output.contains("MA CORE PERFORMANCE BENCHMARK SUITE"),
        "missing suite header: {output}"
    );
    assert!(
        output.contains("Insufficient test data"),
        "expected 'Insufficient test data' message for empty db: {output}"
    );
    // Verify graph benchmark section exists (resilient to label renumbering)
    assert!(
        output.contains("CTE vs N+1 Pattern Benchmark"),
        "missing graph benchmark header: {output}"
    );
    assert!(
        output.contains("System Health Report"),
        "missing system health section on early return: {output}"
    );

    // Negative gate: empty DB must NOT produce timing statistics
    assert!(
        !output.contains("Hybrid Average:"),
        "empty db should not produce search timing statistics: {output}"
    );
    assert!(
        !output.contains("Sync Min/Max/Median:"),
        "empty db should not produce sync timing statistics: {output}"
    );
}

// RTM-035 / TC-5.5
// T-12.7: RTM FR-5 — seeded DB must produce numeric timing statistics in all sections.
#[test]
fn benchmark_with_data_contains_timing_statistics() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);

    // Seed above the BENCHMARK_MIN_FILES threshold to ensure timing output
    seed_file_content_bulk(&conn, BENCHMARK_MIN_FILES + 10);
    // 3 hub concepts × 8 edges each = 24 edges; hubs exceed the minimum_relationships=5
    // threshold required by `top_graph_concepts` for graph benchmarking
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

    // includeDeepTraversal: true — assert its output section below
    let output = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": true
    }));

    // --- Section presence ---
    assert!(
        output.contains("Search Performance Benchmark"),
        "missing search benchmark section: {output}"
    );
    assert!(
        output.contains("Sync Performance Benchmark"),
        "missing sync benchmark section: {output}"
    );
    assert!(
        output.contains("CTE vs N+1 Pattern Benchmark"),
        "missing graph benchmark section in seeded run: {output}"
    );
    assert!(
        output.contains("Complex Traversal Bottleneck Analysis"),
        "missing deep traversal section despite includeDeepTraversal=true: {output}"
    );

    // --- Statistical labels present ---
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
        output.contains("Sync Min/Max/Median:"),
        "missing Sync Min/Max/Median: {output}"
    );

    // --- Numeric timing validation: verify actual numeric values with ms unit ---
    let timing_re = Regex::new(r"\d+\.\d+ms").expect("valid regex");
    assert!(
        timing_re.is_match(&output),
        "expected numeric timing values like '12.34ms' in output: {output}"
    );

    // Verify that statistical labels are followed by numeric values (not error strings)
    let stat_value_re = Regex::new(r"Hybrid Average: \d+\.\d+ms").expect("valid regex");
    assert!(
        stat_value_re.is_match(&output),
        "Hybrid Average should be followed by a numeric ms value: {output}"
    );

    // Deep traversal section should not say "Skipped" since we set includeDeepTraversal=true
    assert!(
        !output.contains("Skipped: includeDeepTraversal=false"),
        "deep traversal should not be skipped when includeDeepTraversal=true: {output}"
    );
}

// RTM-036 / TC-5.6
// T-12.7: RTM FR-5 — learn=true returns documentation, not live benchmark output.
#[test]
fn benchmark_learn_true_returns_help_documentation() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "learn": true }));

    assert!(
        output.contains("RunFullBenchmark"),
        "expected tool name in help output: {output}"
    );

    // Help doc uses lowercase parameter names consistently (pinned, not hedged)
    let output_lower = output.to_lowercase();
    assert!(
        output_lower.contains("iterations"),
        "expected 'iterations' parameter in help output: {output}"
    );
    assert!(
        output_lower.contains("includedeeptraversal"),
        "expected 'includeDeepTraversal' parameter in help output: {output}"
    );

    // Negative gate: learn=true must not produce live benchmark statistics.
    // The help doc contains example output inside ```text blocks, so we check
    // outside code blocks only to avoid false positives from documentation examples.
    let outside_code_blocks: String = output.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("Hybrid Average:"),
        "learn=true should not produce live search statistics outside code blocks: {output}"
    );
    assert!(
        !outside_code_blocks.contains("Sync Min/Max/Median:"),
        "learn=true should not produce live sync statistics outside code blocks: {output}"
    );
}
