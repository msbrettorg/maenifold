use crate::error::Result;
use crate::markdown::normalize_concept;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub name: String,
    pub file_count: i64,
    pub community_id: Option<i64>,
    pub degree: i64,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: i64,
}

/// Collect all concept names reachable from `center` within `depth` hops via BFS,
/// mirroring the frontier-expansion pattern used by `expand_related_concepts` in
/// `graph::build_context`.
fn bfs_ego_set(conn: &Connection, center: &str, depth: u32) -> Result<HashSet<String>> {
    let mut visited = HashSet::new();
    visited.insert(center.to_string());
    let mut frontier = vec![center.to_string()];

    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut next_frontier = Vec::new();
        for concept in &frontier {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT \
                     CASE WHEN concept_a = ?1 THEN concept_b ELSE concept_a END AS neighbor \
                 FROM concept_graph \
                 WHERE concept_a = ?1 OR concept_b = ?1",
            )?;
            let rows = stmt.query_map(params![concept], |row| row.get::<_, String>(0))?;
            for row in rows {
                let neighbor = row?;
                if visited.insert(neighbor.clone()) {
                    next_frontier.push(neighbor);
                }
            }
        }
        frontier = next_frontier;
    }

    Ok(visited)
}

/// Return graph nodes with derived `file_count`, `degree`, and optional `community_id`.
///
/// * `community` – if provided, only nodes belonging to that community are returned.
/// * `center` + `depth` – if provided, return an ego-graph (BFS neighbourhood) rooted
///   at `center` up to `depth` hops, consistent with `BuildContext` traversal semantics.
///   When both `community` and ego-graph parameters are supplied the ego-graph is
///   computed first and then filtered to the requested community.
pub fn get_nodes(
    conn: &Connection,
    community: Option<i64>,
    center: Option<&str>,
    depth: Option<u32>,
) -> Result<Vec<GraphNode>> {
    // Optionally compute the ego-graph node set via BFS.
    let ego_set = if let Some(c) = center {
        let normalized = normalize_concept(c);
        Some(bfs_ego_set(conn, &normalized, depth.unwrap_or(2))?)
    } else {
        None
    };

    let mut stmt = conn.prepare(
        "SELECT c.concept_name, \
                COALESCE(fc.file_count, 0), \
                cc.community_id, \
                COALESCE(deg.degree, 0) \
         FROM concepts c \
         LEFT JOIN ( \
             SELECT concept_name, COUNT(DISTINCT source_file) AS file_count \
             FROM concept_mentions \
             GROUP BY concept_name \
         ) fc ON fc.concept_name = c.concept_name \
         LEFT JOIN concept_communities cc ON cc.concept_name = c.concept_name \
         LEFT JOIN ( \
             SELECT name, COUNT(*) AS degree FROM ( \
                 SELECT concept_a AS name FROM concept_graph \
                 UNION ALL \
                 SELECT concept_b AS name FROM concept_graph \
             ) GROUP BY name \
         ) deg ON deg.name = c.concept_name \
         ORDER BY c.concept_name ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(GraphNode {
            name: row.get(0)?,
            file_count: row.get(1)?,
            community_id: row.get(2)?,
            degree: row.get(3)?,
        })
    })?;

    let mut nodes = Vec::new();
    for row in rows {
        let node = row?;
        if let Some(cid) = community
            && node.community_id != Some(cid)
        {
            continue;
        }
        if let Some(ref ego) = ego_set
            && !ego.contains(&node.name)
        {
            continue;
        }
        nodes.push(node);
    }

    Ok(nodes)
}

/// Return graph edges mapping `concept_a` → `source`, `concept_b` → `target`,
/// `co_occurrence_count` → `weight`.
///
/// If `community` is provided, only edges where **both** endpoints belong to that
/// community are included.
pub fn get_edges(conn: &Connection, community: Option<i64>) -> Result<Vec<GraphEdge>> {
    let edges = if let Some(cid) = community {
        let mut stmt = conn.prepare(
            "SELECT g.concept_a, g.concept_b, COALESCE(g.co_occurrence_count, 0) \
             FROM concept_graph g \
             JOIN concept_communities cc_a \
                 ON cc_a.concept_name = g.concept_a AND cc_a.community_id = ?1 \
             JOIN concept_communities cc_b \
                 ON cc_b.concept_name = g.concept_b AND cc_b.community_id = ?1 \
             ORDER BY g.co_occurrence_count DESC, g.concept_a ASC, g.concept_b ASC",
        )?;
        stmt.query_map(params![cid], |row| {
            Ok(GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0) \
             FROM concept_graph \
             ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC",
        )?;
        stmt.query_map([], |row| {
            Ok(GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    Ok(edges)
}
