// T-01.16: random walk traversal extracted from graph/analyzer.rs.
// No behavioural changes.

use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::error::Result;
use crate::markdown::normalize_concept;

// T-1.6: RTM NFR-1 - port the C# GraphAnalyzer.RandomWalk helper for graph traversal parity.
pub fn random_walk(
    connection: &Connection,
    start_concept: &str,
    walk_length: Option<usize>,
) -> Result<Vec<String>> {
    let start_concept = normalize_concept(start_concept);
    let walk_length = walk_length.unwrap_or(10).max(1);
    let mut walk = vec![start_concept.clone()];
    let mut current = start_concept;

    for _ in 1..walk_length {
        let mut statement = connection.prepare(
            "SELECT DISTINCT \
                 CASE WHEN concept_a = ?1 THEN concept_b ELSE concept_a END AS neighbor \
             FROM concept_graph \
             WHERE concept_a = ?1 OR concept_b = ?1 \
             ORDER BY neighbor ASC",
        )?;
        let rows = statement.query_map(params![current], |row| row.get::<_, String>(0))?;
        let neighbors = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        if neighbors.is_empty() {
            break;
        }

        let index = (Uuid::new_v4().as_u128() % neighbors.len() as u128) as usize;
        current = neighbors[index].clone();
        walk.push(current.clone());
    }

    Ok(walk)
}
