use super::stats::TimingStats;
use crate::benchmark::render::format_ms;
use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::{BTreeSet, VecDeque};

// T-12.6: RTM FR-5
pub(crate) fn render_graph_benchmark(connection: &Connection, iterations: usize) -> Result<String> {
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
            let started = std::time::Instant::now();
            let _ = traverse_graph_cte(connection, concept, 3, 50)?;
            cte_timings.push(started.elapsed().as_secs_f64() * 1_000.0);

            let started = std::time::Instant::now();
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

// T-12.6: RTM FR-5
pub(crate) fn render_complex_traversal_benchmark(
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
            let started = std::time::Instant::now();
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

pub(crate) fn top_graph_concepts(
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

pub(crate) fn traverse_graph_cte(
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

pub(crate) fn traverse_graph_n_plus_one(
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
