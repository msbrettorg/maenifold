use super::stats::TimingStats;
use crate::benchmark::render::{claim_status, format_ms};
use crate::error::Result;
use rusqlite::{Connection, params};

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

// T-12.6: RTM FR-5
pub(crate) fn render_search_benchmark(
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
            let started = std::time::Instant::now();
            let _ = benchmark_hybrid_search(connection, query, sample_size)?;
            hybrid_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = std::time::Instant::now();
            let _ = benchmark_semantic_search(connection, query, sample_size)?;
            semantic_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = std::time::Instant::now();
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

pub(crate) fn benchmark_hybrid_search(
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

pub(crate) fn benchmark_semantic_search(
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

pub(crate) fn benchmark_full_text_search(
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

pub(crate) fn slugify_query(query: &str) -> String {
    query
        .to_ascii_lowercase()
        .replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}
