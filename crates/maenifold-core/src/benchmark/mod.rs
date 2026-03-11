mod graph;
mod helpers;
mod search;
mod stats;
mod sync_runtime;
// T-12.0: RTM NFR-1 preserve the benchmark facade while moving render helpers under benchmark/.
mod render;
#[allow(unused_imports)]
pub use render::*;

use crate::benchmark::render::{
    query_scalar_i64, render_insufficient_search_data_report, render_output, render_system_health,
    timestamp_now,
};
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use graph::{render_complex_traversal_benchmark, render_graph_benchmark};
use helpers::load_help_doc;
use search::render_search_benchmark;
use serde_json::Value;
use sync_runtime::render_sync_benchmark;

// T-12.6: RTM FR-5
fn benchmark_help_output(context: &ToolContext, payload: &Value) -> Option<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !learn {
        return None;
    }

    let help = load_help_doc(&context.config.assets_path, "runfullbenchmark")
        .unwrap_or_else(|| "ERROR: Help file not found for RunFullBenchmark".to_string());
    Some(render_output(context.output_format, help))
}

// T-12.6: RTM FR-5
fn parse_benchmark_iterations(payload: &Value) -> Result<usize> {
    let iterations = payload
        .get("iterations")
        .and_then(Value::as_i64)
        .unwrap_or(5);
    if iterations <= 0 {
        return Err(MaenifoldError::InvalidArgument(
            "iterations must be greater than zero".to_string(),
        ));
    }
    // T-2.6: RTM FR-5 / RTM-032 bound RunFullBenchmark iterations to prevent DoS.
    if iterations > 100 {
        return Err(MaenifoldError::InvalidArgument(
            "iterations must be between 1 and 100".to_string(),
        ));
    }
    Ok(iterations as usize)
}

// T-2.2: RTM FR-5
pub fn run_full_benchmark(context: &ToolContext, payload: &Value) -> Result<String> {
    if let Some(help_output) = benchmark_help_output(context, payload) {
        return Ok(help_output);
    }

    let iterations = parse_benchmark_iterations(payload)?;
    let max_test_files = payload
        .get("maxTestFiles")
        .and_then(Value::as_i64)
        .unwrap_or(1_000)
        .max(0) as usize;
    let include_deep_traversal = payload
        .get("includeDeepTraversal")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;

    let started = timestamp_now();
    let mut report = format!(
        "MA CORE PERFORMANCE BENCHMARK SUITE\n\
=====================================\n\
Iterations: {iterations}, Max Test Files: {max_test_files}\n\
Started: {started}\n\n"
    );

    // T-2.2.1: RTM-031, RTM-035
    report.push_str(&render_graph_benchmark(&connection, iterations)?);

    let indexed_file_count = query_scalar_i64(&connection, "SELECT COUNT(*) FROM file_content")?;
    let sampled_file_count = indexed_file_count.min(max_test_files as i64);
    if sampled_file_count < 100 {
        return render_insufficient_search_data_report(
            context,
            report,
            indexed_file_count,
            sampled_file_count,
        );
    }

    // T-2.2.2: RTM-031, RTM-032, RTM-033, RTM-035, RTM-037
    report.push_str(&render_search_benchmark(
        &connection,
        iterations,
        max_test_files,
        indexed_file_count as usize,
    )?);

    // T-2.2.3: RTM-031, RTM-032, RTM-035
    report.push_str(&render_sync_benchmark(context, iterations)?);

    // T-2.2.4: RTM-031, RTM-034, RTM-035
    report.push_str(&render_complex_traversal_benchmark(
        &connection,
        iterations,
        include_deep_traversal,
    )?);

    // T-2.2.5: RTM-031
    report.push_str(&render_system_health(context)?);
    report.push_str(&format!("\nCompleted: {}\n", timestamp_now()));

    Ok(render_output(context.output_format, report))
}
