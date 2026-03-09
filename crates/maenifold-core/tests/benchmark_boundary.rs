use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

fn invoke_benchmark(payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke("RunFullBenchmark", &payload, OutputFormat::Markdown)
        .expect("invoke RunFullBenchmark")
}

fn invoke_benchmark_result(
    payload: serde_json::Value,
) -> Result<String, maenifold_core::MaenifoldError> {
    ToolRegistry::global().try_invoke("RunFullBenchmark", &payload, OutputFormat::Markdown)
}

/// Seed `count` rows into file_content (triggers auto-populate file_search via FTS triggers).
fn seed_file_content_bulk(conn: &Connection, count: usize) {
    for i in 0..count {
        conn.execute(
            "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("memory://benchmark-test/file-{i}"),
                format!("Benchmark File {i}"),
                format!(
                    "This file contains content about machine learning algorithms and \
                     knowledge graph structures for semantic search testing. \
                     Unique token: token_{i}."
                ),
                format!("md5-bench-{i}"),
                100i64 + i as i64,
                "2026-06-01T00:00:00Z",
                "active"
            ],
        )
        .unwrap_or_else(|e| panic!("seed file_content row {i}: {e}"));
    }
}

/// Seed enough graph data (concepts + edges) so CTE/N+1 benchmarks fire.
fn seed_graph_data(conn: &Connection, hub_count: usize, edges_per_hub: usize) {
    for h in 0..hub_count {
        let hub = format!("hub-concept-{h}");
        conn.execute(
            "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
            params![hub],
        )
        .expect("insert hub concept");
        for e in 0..edges_per_hub {
            let leaf = format!("leaf-{h}-{e}");
            conn.execute(
                "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
                params![leaf],
            )
            .expect("insert leaf concept");
            conn.execute(
                "INSERT OR IGNORE INTO concept_graph (concept_a, concept_b, co_occurrence_count)
                 VALUES (?1, ?2, 1)",
                params![hub, leaf],
            )
            .expect("insert edge");
        }
    }
}

/// Write a markdown file into the memory directory so Sync can index it.
fn write_memory_file(cfg: &Config, relative: &str, content: &str) {
    let path = cfg.memory_path.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write markdown");
}

// ---------------------------------------------------------------------------
// TC-5.1 / RTM-031, RTM-037: Empty DB — section headers OR "Insufficient test data"
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-031 / TC-5.1)
#[test]
fn benchmark_empty_db_contains_section_headers_or_insufficient_data() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_benchmark(json!({ "iterations": 1 }));

    // Must contain the suite header
    assert!(
        output.contains("MA CORE PERFORMANCE BENCHMARK SUITE"),
        "missing suite header: {output}"
    );

    // With zero files, it should hit the <100 threshold → "Insufficient test data"
    // T-2.5b: RTM FR-5 (RTM-037 / TC-5.1)
    assert!(
        output.contains("Insufficient test data"),
        "expected 'Insufficient test data' message for empty db: {output}"
    );

    // The graph benchmark section should still appear (it handles empty graph gracefully)
    assert!(
        output.contains("GRPH-009: CTE vs N+1 Pattern Benchmark"),
        "missing graph benchmark header: {output}"
    );

    // System health should still appear even on early return
    assert!(
        output.contains("System Health Report"),
        "missing system health section on early return: {output}"
    );
}

// ---------------------------------------------------------------------------
// TC-5.2 / RTM-032: iterations parameter reflected in output
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-032 / TC-5.2)
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

// ---------------------------------------------------------------------------
// TC-5.3 / RTM-033: maxTestFiles reflected in output
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-033 / TC-5.3)
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

// ---------------------------------------------------------------------------
// TC-5.4 / RTM-034: includeDeepTraversal=false skips complex traversal
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-034 / TC-5.4)
#[test]
fn benchmark_exclude_deep_traversal_skips_complex_section_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);

    // Seed enough data to get past the <100 file threshold and seed graph data
    seed_file_content_bulk(&conn, 110);
    seed_graph_data(&conn, 5, 10);

    let output = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": false
    }));

    // The section header still appears but the content should say "Skipped"
    assert!(
        output.contains("Complex Traversal Bottleneck Analysis"),
        "expected section header present: {output}"
    );
    assert!(
        output.contains("Skipped: includeDeepTraversal=false"),
        "expected skip message: {output}"
    );

    // When deep traversal is included, it should NOT say "Skipped"
    let output_included = invoke_benchmark(json!({
        "iterations": 1,
        "includeDeepTraversal": true
    }));
    assert!(
        !output_included.contains("Skipped: includeDeepTraversal=false"),
        "should not contain skip message when deep traversal is enabled: {output_included}"
    );
}

// ---------------------------------------------------------------------------
// TC-5.5 / RTM-035: Output contains min/max/avg/median timing stats
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-035 / TC-5.5)
#[test]
fn benchmark_with_data_contains_timing_statistics() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);

    // Seed 110 file_content rows to pass the >=100 threshold
    seed_file_content_bulk(&conn, 110);
    // Seed graph data for CTE/N+1 benchmarks
    seed_graph_data(&conn, 3, 8);

    // Also write some physical markdown files so Sync has something to process
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

    // Search benchmark section should have min/max/median stats
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

    // Sync benchmark section should have timing stats
    assert!(
        output.contains("Sync Performance Benchmark"),
        "missing sync benchmark section: {output}"
    );
    assert!(
        output.contains("Sync Min/Max/Median:"),
        "missing Sync Min/Max/Median: {output}"
    );

    // Verify the output contains "ms" timing values (format_ms produces "X.XXms" strings)
    assert!(
        output.contains("ms"),
        "expected 'ms' unit in timing output: {output}"
    );
}

// ---------------------------------------------------------------------------
// TC-5.6 / RTM-036: learn=true returns help documentation
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-036 / TC-5.6)
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
    // The help doc contains the suite header inside a code block example, so we
    // verify learn=true did NOT produce a live "Started:" timestamp line, which
    // only appears in actual benchmark runs (outside code blocks).
    let outside_code_blocks: String = output
        .split("```")
        .step_by(2) // odd-indexed chunks are inside code blocks
        .collect();
    assert!(
        !outside_code_blocks.contains("Started: 20"),
        "learn=true should not produce live benchmark output: {output}"
    );
}

// ---------------------------------------------------------------------------
// Additional edge-case: iterations=0 returns error
// ---------------------------------------------------------------------------

// T-2.5b: RTM FR-5 (RTM-032 / TC-5.2 — invalid iterations)
#[test]
fn benchmark_iterations_zero_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_benchmark_result(json!({ "iterations": 0 }));
    assert!(result.is_err(), "iterations=0 should return an error");
    let err = result.unwrap_err();
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("iterations must be greater than zero"),
        "unexpected error message: {err_msg}"
    );
}
