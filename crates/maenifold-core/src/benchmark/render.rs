use crate::db;
use crate::error::Result;
use crate::markdown::drain_frontmatter_warnings;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use rusqlite::Connection;
use std::fs;

pub(crate) fn render_insufficient_search_data_report(
    context: &ToolContext,
    mut report: String,
    indexed_file_count: i64,
    sampled_file_count: i64,
) -> Result<String> {
    report.push_str("Search Performance Benchmark\n");
    report.push_str("----------------------------\n");
    report.push_str(&format!(
        "Insufficient test data ({} files indexed, sample size {}). Need at least 100 files.\n\n",
        indexed_file_count, sampled_file_count
    ));
    report.push_str(&render_system_health(context)?);
    report.push_str(&format!("\nCompleted: {}\n", timestamp_now()));
    Ok(render_output(context.output_format, report))
}

pub(crate) fn render_system_health(context: &ToolContext) -> Result<String> {
    let connection = db::open_connection(&context.config.database_path)?;
    let concept_count = query_scalar_i64(&connection, "SELECT COUNT(*) FROM concepts")?;
    let relation_count = query_scalar_i64(&connection, "SELECT COUNT(*) FROM concept_graph")?;
    let file_count = query_scalar_i64(&connection, "SELECT COUNT(*) FROM file_content")?;
    let database_size = fs::metadata(&context.config.database_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let vector_embeddings = match query_vector_counts(&connection) {
        Some((concept_embeddings, file_embeddings)) => {
            format!("{concept_embeddings} concepts, {file_embeddings} files")
        }
        None => "unavailable".to_string(),
    };

    let mut report = format!(
        "System Health Report\n\
--------------------\n\
Database health:\n\
  Concepts: {concept_count}\n\
  Relations: {relation_count}\n\
  Files indexed: {file_count}\n\
  Vector embeddings: {vector_embeddings}\n\
  Database size: {}\n\n",
        human_bytes(database_size),
    );

    let health = crate::sync::watchers::watcher_health();
    report.push_str(&format!(
        "Watcher health:\n\
         \x20 Status: {}\n\
         \x20 Total sync failures: {}\n\
         \x20 Consecutive failures: {}\n\
         \x20 Last error: {}\n\
         \x20 Changes processed: {}\n\n",
        if health.is_running {
            "running"
        } else {
            "stopped"
        },
        health.total_failures,
        health.consecutive_failures,
        health.last_error.as_deref().unwrap_or("none"),
        health.changes_processed,
    ));

    let warnings = drain_frontmatter_warnings();
    if !warnings.is_empty() {
        report.push_str(&format!("Frontmatter warnings: {} files\n", warnings.len()));
        for (uri, error) in &warnings {
            report.push_str(&format!("  {uri}: {error}\n"));
        }
        report.push('\n');
    }

    Ok(report)
}

fn query_vector_counts(connection: &Connection) -> Option<(i64, i64)> {
    let concept_embeddings = connection
        .query_row("SELECT COUNT(*) FROM vec_concepts", [], |row| {
            row.get::<_, i64>(0)
        })
        .ok()?;
    let file_embeddings = connection
        .query_row("SELECT COUNT(*) FROM vec_memory_files", [], |row| {
            row.get::<_, i64>(0)
        })
        .ok()?;
    Some((concept_embeddings, file_embeddings))
}

pub(crate) fn query_scalar_i64(connection: &Connection, sql: &str) -> Result<i64> {
    connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(Into::into)
}

pub(crate) fn render_output(format: OutputFormat, report: String) -> String {
    match format {
        OutputFormat::Json => success(report),
        OutputFormat::Markdown => report,
    }
}

pub(crate) fn claim_status(actual_ms: f64, claim_ms: f64) -> &'static str {
    if claim_ms <= 0.0 {
        return "n/a";
    }
    let pct = ((actual_ms - claim_ms).abs() / claim_ms) * 100.0;
    if pct <= 20.0 {
        "✓"
    } else if pct <= 50.0 {
        "⚠"
    } else {
        "✗"
    }
}

pub(crate) fn format_ms(value: f64) -> String {
    format!("{value:.2}ms")
}

pub(crate) fn format_seconds(value_ms: f64) -> String {
    format!("{:.2}s", value_ms / 1_000.0)
}

fn human_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    format!("{value:.1} {}", UNITS[unit_index])
}

pub(crate) fn timestamp_now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
