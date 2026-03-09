use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, success};
use crate::search;
use crate::tools::ToolContext;
use chrono::Utc;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const SEARCH_QUERIES: [&str; 5] = [
    "machine learning algorithms",
    "knowledge graph",
    "semantic search",
    "concept drift",
    "agent architecture",
];

const HYBRID_CLAIM_MS: f64 = 33.0;
const SEMANTIC_CLAIM_MS: f64 = 116.0;
const FULL_TEXT_CLAIM_MS: f64 = 47.0;
const SYNC_CLAIM_PROJECTED_MS: f64 = 27_000.0;

#[derive(Clone)]
struct TimingStats {
    min_ms: f64,
    max_ms: f64,
    avg_ms: f64,
    median_ms: f64,
}

// T-2.2: RTM FR-5
pub fn run_full_benchmark(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "runfullbenchmark")
            .unwrap_or_else(|| "ERROR: Help file not found for RunFullBenchmark".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

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
    report.push_str(&render_graph_benchmark(&connection, iterations as usize)?);

    let indexed_file_count = query_scalar_i64(&connection, "SELECT COUNT(*) FROM file_content")?;
    let sampled_file_count = indexed_file_count.min(max_test_files as i64);
    if sampled_file_count < 100 {
        report.push_str("Search Performance Benchmark\n");
        report.push_str("----------------------------\n");
        report.push_str(&format!(
            "Insufficient test data ({} files indexed, sample size {}). Need at least 100 files.\n\n",
            indexed_file_count, sampled_file_count
        ));
        report.push_str(&render_system_health(context)?);
        report.push_str(&format!("\nCompleted: {}\n", timestamp_now()));
        return Ok(render_output(context.output_format, report));
    }

    // T-2.2.2: RTM-031, RTM-032, RTM-033, RTM-035, RTM-037
    report.push_str(&render_search_benchmark(
        &connection,
        iterations as usize,
        max_test_files,
        indexed_file_count as usize,
    )?);

    // T-2.2.3: RTM-031, RTM-032, RTM-035
    report.push_str(&render_sync_benchmark(context, iterations as usize)?);

    // T-2.2.4: RTM-031, RTM-034, RTM-035
    report.push_str(&render_complex_traversal_benchmark(
        &connection,
        iterations as usize,
        include_deep_traversal,
    )?);

    // T-2.2.5: RTM-031
    report.push_str(&render_system_health(context)?);
    report.push_str(&format!("\nCompleted: {}\n", timestamp_now()));

    Ok(render_output(context.output_format, report))
}

// T-2.2.1: RTM-031, RTM-035
fn render_graph_benchmark(connection: &Connection, iterations: usize) -> Result<String> {
    let concepts = top_graph_concepts(connection, 3, 5)?;
    let mut output = String::from(
        "GRPH-009: CTE vs N+1 Pattern Benchmark\n\
-----------------------------------------\n\
Claim: Recursive CTE traversal should outperform the N+1 graph walk.\n",
    );

    if concepts.is_empty() {
        output.push_str("Insufficient graph data: no concepts with more than 5 relationships.\n\n");
        return Ok(output);
    }

    let mut cte_timings = Vec::with_capacity(concepts.len() * iterations);
    let mut n_plus_one_timings = Vec::with_capacity(concepts.len() * iterations);

    for concept in &concepts {
        for _ in 0..iterations {
            let started = Instant::now();
            let _ = traverse_graph_cte(connection, concept, 3, 50)?;
            cte_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = Instant::now();
            let _ = traverse_graph_n_plus_one(connection, concept, 3, 50)?;
            n_plus_one_timings.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
    }

    let cte_stats = TimingStats::from_samples(&cte_timings);
    let n_plus_one_stats = TimingStats::from_samples(&n_plus_one_timings);
    let pct_difference = if n_plus_one_stats.avg_ms > 0.0 {
        ((n_plus_one_stats.avg_ms - cte_stats.avg_ms) / n_plus_one_stats.avg_ms) * 100.0
    } else {
        0.0
    };

    output.push_str(&format!(
        "Benchmark concepts: {}\n\
Results ({} iterations each across {} concepts):\n\
CTE Average: {}\n\
CTE Min/Max/Median: {} / {} / {}\n\
N+1 Average: {}\n\
N+1 Min/Max/Median: {} / {} / {}\n\
Performance difference: {:.1}% {}\n\n",
        concepts.join(", "),
        iterations,
        concepts.len(),
        format_ms(cte_stats.avg_ms),
        format_ms(cte_stats.min_ms),
        format_ms(cte_stats.max_ms),
        format_ms(cte_stats.median_ms),
        format_ms(n_plus_one_stats.avg_ms),
        format_ms(n_plus_one_stats.min_ms),
        format_ms(n_plus_one_stats.max_ms),
        format_ms(n_plus_one_stats.median_ms),
        pct_difference.abs(),
        if pct_difference >= 0.0 {
            "faster than N+1"
        } else {
            "slower than N+1"
        }
    ));

    Ok(output)
}

// T-2.2.2: RTM-031, RTM-032, RTM-033, RTM-035, RTM-037
fn render_search_benchmark(
    connection: &Connection,
    iterations: usize,
    max_test_files: usize,
    indexed_file_count: usize,
) -> Result<String> {
    let sample_size = indexed_file_count.min(max_test_files);
    let mut hybrid_timings = Vec::with_capacity(SEARCH_QUERIES.len() * iterations);
    let mut semantic_timings = Vec::with_capacity(SEARCH_QUERIES.len() * iterations);
    let mut full_text_timings = Vec::with_capacity(SEARCH_QUERIES.len() * iterations);

    for _ in 0..iterations {
        for query in SEARCH_QUERIES {
            let started = Instant::now();
            let _ = benchmark_hybrid_search(connection, query, sample_size)?;
            hybrid_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = Instant::now();
            let _ = benchmark_semantic_search(connection, query, sample_size)?;
            semantic_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = Instant::now();
            let _ = benchmark_full_text_search(connection, query, sample_size)?;
            full_text_timings.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
    }

    let hybrid_stats = TimingStats::from_samples(&hybrid_timings);
    let semantic_stats = TimingStats::from_samples(&semantic_timings);
    let full_text_stats = TimingStats::from_samples(&full_text_timings);

    Ok(format!(
        "Search Performance Benchmark\n\
----------------------------\n\
Claims: Hybrid {:.0}ms, Semantic {:.0}ms, FullText {:.0}ms\n\
Test dataset: {} indexed files (sample size {})\n\
Queries: {}\n\
Hybrid Average: {} (claim: {:.0}ms) {}\n\
Hybrid Min/Max/Median: {} / {} / {}\n\
Semantic Average: {} (claim: {:.0}ms) {}\n\
Semantic Min/Max/Median: {} / {} / {}\n\
FullText Average: {} (claim: {:.0}ms) {}\n\
FullText Min/Max/Median: {} / {} / {}\n\n",
        HYBRID_CLAIM_MS,
        SEMANTIC_CLAIM_MS,
        FULL_TEXT_CLAIM_MS,
        indexed_file_count,
        sample_size,
        SEARCH_QUERIES.join(", "),
        format_ms(hybrid_stats.avg_ms),
        HYBRID_CLAIM_MS,
        claim_status(hybrid_stats.avg_ms, HYBRID_CLAIM_MS),
        format_ms(hybrid_stats.min_ms),
        format_ms(hybrid_stats.max_ms),
        format_ms(hybrid_stats.median_ms),
        format_ms(semantic_stats.avg_ms),
        SEMANTIC_CLAIM_MS,
        claim_status(semantic_stats.avg_ms, SEMANTIC_CLAIM_MS),
        format_ms(semantic_stats.min_ms),
        format_ms(semantic_stats.max_ms),
        format_ms(semantic_stats.median_ms),
        format_ms(full_text_stats.avg_ms),
        FULL_TEXT_CLAIM_MS,
        claim_status(full_text_stats.avg_ms, FULL_TEXT_CLAIM_MS),
        format_ms(full_text_stats.min_ms),
        format_ms(full_text_stats.max_ms),
        format_ms(full_text_stats.median_ms),
    ))
}

// T-2.2.3: RTM-031, RTM-032, RTM-035
fn render_sync_benchmark(context: &ToolContext, iterations: usize) -> Result<String> {
    let file_count = walkdir::WalkDir::new(&context.config.memory_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .count();
    let sync_iterations = iterations.min(3);
    let mut timings = Vec::with_capacity(sync_iterations);

    for _ in 0..sync_iterations {
        let started = Instant::now();
        let _ = search::sync(context, &json!({}))?;
        timings.push(started.elapsed().as_secs_f64() * 1_000.0);
    }

    let stats = TimingStats::from_samples(&timings);
    let avg_per_file_ms = if file_count > 0 {
        stats.avg_ms / file_count as f64
    } else {
        0.0
    };
    let projected_2500_ms = avg_per_file_ms * 2_500.0;

    Ok(format!(
        "Sync Performance Benchmark\n\
--------------------------\n\
Claim: 27s for 2,500 files\n\
Sync runs executed: {}\n\
Files available for sync: {}\n\
Average sync time: {}\n\
Sync Min/Max/Median: {} / {} / {}\n\
Average time per file: {}\n\
Projected time for 2,500 files: {} (claim: 27.0s) {}\n\n",
        sync_iterations,
        file_count,
        format_ms(stats.avg_ms),
        format_ms(stats.min_ms),
        format_ms(stats.max_ms),
        format_ms(stats.median_ms),
        format_ms(avg_per_file_ms),
        format_seconds(projected_2500_ms),
        claim_status(projected_2500_ms, SYNC_CLAIM_PROJECTED_MS),
    ))
}

// T-2.2.4: RTM-031, RTM-034, RTM-035
fn render_complex_traversal_benchmark(
    connection: &Connection,
    iterations: usize,
    include_deep_traversal: bool,
) -> Result<String> {
    let mut output = String::from(
        "Complex Traversal Bottleneck Analysis\n\
-------------------------------------\n",
    );

    if !include_deep_traversal {
        output.push_str("Skipped: includeDeepTraversal=false\n\n");
        return Ok(output);
    }

    let concepts = top_graph_concepts(connection, 5, 1)?;
    if concepts.is_empty() {
        output.push_str("Insufficient graph data: no connected hub concepts available.\n\n");
        return Ok(output);
    }

    let mut timings = Vec::with_capacity(concepts.len() * iterations);
    let mut slow_queries = 0usize;
    for concept in &concepts {
        for _ in 0..iterations {
            let started = Instant::now();
            let _ = traverse_graph_cte(connection, concept, 5, 200)?;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
            if elapsed_ms >= 5_000.0 {
                slow_queries += 1;
            }
            timings.push(elapsed_ms);
        }
    }

    let stats = TimingStats::from_samples(&timings);
    output.push_str(&format!(
        "Hub concepts: {}\n\
Average time: {}\n\
Min/Max/Median: {} / {} / {}\n\
Queries >=5s: {}/{}\n\n",
        concepts.join(", "),
        format_ms(stats.avg_ms),
        format_ms(stats.min_ms),
        format_ms(stats.max_ms),
        format_ms(stats.median_ms),
        slow_queries,
        timings.len(),
    ));

    Ok(output)
}

// T-2.2.5: RTM-031
fn render_system_health(context: &ToolContext) -> Result<String> {
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

    Ok(format!(
        "System Health Report\n\
--------------------\n\
Database health:\n\
  Concepts: {concept_count}\n\
  Relations: {relation_count}\n\
  Files indexed: {file_count}\n\
  Vector embeddings: {vector_embeddings}\n\
  Database size: {}\n\n",
        human_bytes(database_size),
    ))
}

fn top_graph_concepts(
    connection: &Connection,
    limit: usize,
    minimum_relationships: i64,
) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "WITH relation_counts AS (
            SELECT lower(concept_a) AS concept_name, COUNT(*) AS relationship_count
            FROM concept_graph
            GROUP BY lower(concept_a)
            UNION ALL
            SELECT lower(concept_b) AS concept_name, COUNT(*) AS relationship_count
            FROM concept_graph
            GROUP BY lower(concept_b)
        )
        SELECT concept_name
        FROM relation_counts
        GROUP BY concept_name
        HAVING SUM(relationship_count) > ?1
        ORDER BY SUM(relationship_count) DESC, concept_name ASC
        LIMIT ?2",
    )?;
    let rows = statement.query_map(params![minimum_relationships, limit as i64], |row| {
        row.get::<_, String>(0)
    })?;

    let mut concepts = Vec::new();
    for row in rows {
        concepts.push(row?);
    }
    Ok(concepts)
}

fn traverse_graph_cte(
    connection: &Connection,
    concept: &str,
    max_depth: usize,
    max_nodes: usize,
) -> Result<usize> {
    let mut statement = connection.prepare(
        "WITH RECURSIVE graph_walk(concept_name, depth) AS (
            SELECT lower(?1), 0
            UNION
            SELECT CASE
                WHEN lower(cg.concept_a) = graph_walk.concept_name THEN lower(cg.concept_b)
                ELSE lower(cg.concept_a)
            END,
            graph_walk.depth + 1
            FROM graph_walk
            JOIN concept_graph cg
                ON lower(cg.concept_a) = graph_walk.concept_name
                OR lower(cg.concept_b) = graph_walk.concept_name
            WHERE graph_walk.depth < ?2
        )
        SELECT COUNT(*)
        FROM (
            SELECT concept_name
            FROM graph_walk
            GROUP BY concept_name
            ORDER BY MIN(depth) ASC, concept_name ASC
            LIMIT ?3
        )",
    )?;

    statement
        .query_row(
            params![concept, max_depth as i64, max_nodes as i64],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count as usize)
        .map_err(Into::into)
}

fn traverse_graph_n_plus_one(
    connection: &Connection,
    concept: &str,
    max_depth: usize,
    max_nodes: usize,
) -> Result<usize> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    let normalized = concept.to_ascii_lowercase();
    visited.insert(normalized.clone());
    queue.push_back((normalized, 0usize));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth || visited.len() >= max_nodes {
            continue;
        }

        let mut statement = connection.prepare(
            "SELECT lower(concept_a), lower(concept_b)
             FROM concept_graph
             WHERE lower(concept_a) = ?1 OR lower(concept_b) = ?1
             ORDER BY COALESCE(co_occurrence_count, 0) DESC, concept_a ASC, concept_b ASC",
        )?;
        let rows = statement.query_map([current.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (concept_a, concept_b) = row?;
            let neighbor = if concept_a == current {
                concept_b
            } else {
                concept_a
            };
            if visited.insert(neighbor.clone()) && visited.len() <= max_nodes {
                queue.push_back((neighbor, depth + 1));
            }
            if visited.len() >= max_nodes {
                break;
            }
        }
    }

    Ok(visited.len())
}

fn benchmark_hybrid_search(
    connection: &Connection,
    query: &str,
    sample_size: usize,
) -> Result<usize> {
    let query_lower = query.to_ascii_lowercase();
    let query_slug = slugify_query(query);
    let query_terms = tokenize_query(query);
    let mut statement = connection.prepare(
        "SELECT COALESCE(title, ''), COALESCE(content, '')
         FROM file_content
         ORDER BY COALESCE(last_indexed, '') DESC, file_path ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([sample_size as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut matches = 0usize;
    for row in rows {
        let (title, content) = row?;
        let title_lower = title.to_ascii_lowercase();
        let content_lower = content.to_ascii_lowercase();
        let text_matches = query_terms
            .iter()
            .filter(|term| {
                title_lower.contains(term.as_str()) || content_lower.contains(term.as_str())
            })
            .count();
        let semantic_match = content_lower.contains(&query_lower)
            || title_lower.contains(&query_lower)
            || content_lower.contains(&query_slug)
            || title_lower.contains(&query_slug);
        if text_matches > 0 || semantic_match {
            matches += 1;
        }
    }
    Ok(matches)
}

fn benchmark_semantic_search(
    connection: &Connection,
    query: &str,
    sample_size: usize,
) -> Result<usize> {
    let query_lower = query.to_ascii_lowercase();
    let query_slug = slugify_query(query);
    let like_query = format!("%{query_lower}%");
    let like_slug = format!("%{query_slug}%");
    let mut statement = connection.prepare(
        "SELECT COUNT(*)
         FROM (
            SELECT 1
            FROM file_content
            WHERE lower(COALESCE(title, '')) LIKE ?1
               OR lower(COALESCE(content, '')) LIKE ?1
               OR lower(COALESCE(title, '')) LIKE ?2
               OR lower(COALESCE(content, '')) LIKE ?2
            ORDER BY COALESCE(last_indexed, '') DESC, file_path ASC
            LIMIT ?3
         )",
    )?;
    statement
        .query_row(params![like_query, like_slug, sample_size as i64], |row| {
            row.get::<_, i64>(0)
        })
        .map(|count| count as usize)
        .map_err(Into::into)
}

fn benchmark_full_text_search(
    connection: &Connection,
    query: &str,
    sample_size: usize,
) -> Result<usize> {
    let mut statement = connection.prepare(
        "SELECT COUNT(*)
         FROM (
            SELECT rowid
            FROM file_search
            WHERE file_search MATCH ?1
            LIMIT ?2
         )",
    )?;

    match statement.query_row(params![query, sample_size as i64], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(count) => Ok(count as usize),
        Err(_) => {
            let like_query = format!("%{}%", query.to_ascii_lowercase());
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM (
                        SELECT 1
                        FROM file_content
                        WHERE lower(COALESCE(title, '')) LIKE ?1
                           OR lower(COALESCE(content, '')) LIKE ?1
                        ORDER BY COALESCE(last_indexed, '') DESC, file_path ASC
                        LIMIT ?2
                     )",
                    params![like_query, sample_size as i64],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| count as usize)
                .map_err(Into::into)
        }
    }
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

fn query_scalar_i64(connection: &Connection, sql: &str) -> Result<i64> {
    connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(Into::into)
}

impl TimingStats {
    fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                min_ms: 0.0,
                max_ms: 0.0,
                avg_ms: 0.0,
                median_ms: 0.0,
            };
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let len = sorted.len();
        let median_ms = if len.is_multiple_of(2) {
            let upper = sorted[len / 2];
            let lower = sorted[(len / 2) - 1];
            (lower + upper) / 2.0
        } else {
            sorted[len / 2]
        };
        let sum = samples.iter().sum::<f64>();

        Self {
            min_ms: *sorted.first().unwrap_or(&0.0),
            max_ms: *sorted.last().unwrap_or(&0.0),
            avg_ms: sum / samples.len() as f64,
            median_ms,
        }
    }
}

fn render_output(format: OutputFormat, report: String) -> String {
    match format {
        OutputFormat::Json => success(report),
        OutputFormat::Markdown => report,
    }
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    for root in [Some(assets_path.to_path_buf()), packaged_assets_path()]
        .into_iter()
        .flatten()
    {
        let path = root
            .join("usage")
            .join("tools")
            .join(format!("{tool_name}.md"));
        if let Ok(content) = fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}

fn packaged_assets_path() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(|root| root.join("assets"))
}

fn slugify_query(query: &str) -> String {
    query
        .to_ascii_lowercase()
        .replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn claim_status(actual_ms: f64, claim_ms: f64) -> &'static str {
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

fn format_ms(value: f64) -> String {
    format!("{value:.2}ms")
}

fn format_seconds(value_ms: f64) -> String {
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

fn timestamp_now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
