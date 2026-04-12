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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityNode {
    pub id: i64,
    pub anchor: String,
    pub size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityEdge {
    pub source: i64,
    pub target: i64,
    pub weight: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityGraph {
    pub nodes: Vec<CommunityNode>,
    pub edges: Vec<CommunityEdge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptNode {
    pub name: String,
    pub occurrences: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptEdge {
    pub source: String,
    pub target: String,
    pub weight: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptGraph {
    pub community_id: i64,
    pub anchor: String,
    pub nodes: Vec<ConceptNode>,
    pub edges: Vec<ConceptEdge>,
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

/// Return decay-weighted graph communities and cross-community edges, preserving the
/// response contract historically served by the desktop Tauri graph commands.
pub fn get_communities(conn: &Connection) -> Result<CommunityGraph> {
    let mut node_stmt = conn.prepare(
        "WITH concept_decay AS (
            SELECT cm.concept_name,
              MAX(
                CASE
                  WHEN julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) <=
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  THEN 1.0
                  ELSE MIN(1.0, 1.0 / sqrt(MAX(1,
                    julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) -
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  )))
                END
              ) as max_decay
            FROM concept_mentions cm
            JOIN file_content fc ON cm.source_file = fc.file_path
            WHERE COALESCE(fc.last_accessed, fc.created, fc.last_indexed) IS NOT NULL
            GROUP BY cm.concept_name
        ),
        community_sizes AS (
            SELECT community_id, COUNT(*) as size
            FROM concept_communities GROUP BY community_id HAVING size >= 3
        ),
        concept_degree AS (
            SELECT cc.concept_name, cc.community_id,
                   SUM(cg.co_occurrence_count) as raw_weight,
                   COALESCE(cd.max_decay, 0.1) as max_decay
            FROM concept_communities cc
            JOIN community_sizes cs ON cc.community_id = cs.community_id
            LEFT JOIN concept_graph cg
              ON cc.concept_name = cg.concept_a OR cc.concept_name = cg.concept_b
            LEFT JOIN concept_decay cd ON cc.concept_name = cd.concept_name
            WHERE cc.concept_name NOT GLOB '*[.]*'
              AND length(cc.concept_name) >= 3
              AND cc.concept_name NOT GLOB '[0-9]*'
            GROUP BY cc.concept_name, cc.community_id
        ),
        ranked AS (
            SELECT *, raw_weight * max_decay as weighted_score,
              ROW_NUMBER() OVER (
                PARTITION BY community_id
                ORDER BY raw_weight * max_decay DESC, concept_name ASC
              ) as rn
            FROM concept_degree WHERE raw_weight > 0
        )
        SELECT r.community_id, r.concept_name as anchor, cs.size
        FROM ranked r
        JOIN community_sizes cs ON r.community_id = cs.community_id
        WHERE r.rn = 1
        ORDER BY cs.size DESC, r.community_id ASC, r.concept_name ASC",
    )?;

    let nodes = node_stmt
        .query_map([], |row| {
            Ok(CommunityNode {
                id: row.get(0)?,
                anchor: row.get(1)?,
                size: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let valid_ids: HashSet<i64> = nodes.iter().map(|node| node.id).collect();

    let mut edge_stmt = conn.prepare(
        "SELECT a_cc.community_id, b_cc.community_id, SUM(cg.co_occurrence_count) as weight
         FROM concept_graph cg
         JOIN concept_communities a_cc ON cg.concept_a = a_cc.concept_name
         JOIN concept_communities b_cc ON cg.concept_b = b_cc.concept_name
         WHERE a_cc.community_id < b_cc.community_id
         GROUP BY a_cc.community_id, b_cc.community_id
         HAVING weight >= 5
         ORDER BY weight DESC, a_cc.community_id ASC, b_cc.community_id ASC
         LIMIT 2000",
    )?;

    let edges = edge_stmt
        .query_map([], |row| {
            Ok(CommunityEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|edge| valid_ids.contains(&edge.source) && valid_ids.contains(&edge.target))
        .collect();

    Ok(CommunityGraph { nodes, edges })
}

/// Return the concepts and intra-community edges for a single graph community,
/// preserving the desktop response shape used by GraphView.
pub fn get_community_concepts(conn: &Connection, community_id: i64) -> Result<ConceptGraph> {
    let anchor = conn
        .query_row(
            "WITH concept_decay AS (
                SELECT cm.concept_name,
                  MAX(
                    CASE
                      WHEN julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) <=
                        CASE
                          WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                          WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                          WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                          ELSE 28
                        END
                      THEN 1.0
                      ELSE MIN(1.0, 1.0 / sqrt(MAX(1,
                        julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) -
                        CASE
                          WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                          WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                          WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                          ELSE 28
                        END
                      )))
                    END
                  ) as max_decay
                FROM concept_mentions cm
                JOIN file_content fc ON cm.source_file = fc.file_path
                WHERE COALESCE(fc.last_accessed, fc.created, fc.last_indexed) IS NOT NULL
                GROUP BY cm.concept_name
            ),
            concept_degree AS (
                SELECT cc.concept_name,
                       SUM(cg.co_occurrence_count) as raw_weight,
                       COALESCE(cd.max_decay, 0.1) as max_decay
                FROM concept_communities cc
                LEFT JOIN concept_graph cg
                    ON cc.concept_name = cg.concept_a OR cc.concept_name = cg.concept_b
                LEFT JOIN concept_decay cd ON cc.concept_name = cd.concept_name
                WHERE cc.community_id = ?1
                  AND cc.concept_name NOT GLOB '*[.]*'
                  AND length(cc.concept_name) >= 3
                GROUP BY cc.concept_name
            )
            SELECT concept_name FROM concept_degree
            WHERE raw_weight > 0
            ORDER BY raw_weight * max_decay DESC, concept_name ASC LIMIT 1",
            params![community_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| format!("community-{community_id}"));

    let mut concept_stmt = conn.prepare(
        "SELECT c.concept_name, COALESCE(c.occurrence_count, 0) as occurrences
         FROM concepts c
         JOIN concept_communities cc ON c.concept_name = cc.concept_name
         WHERE cc.community_id = ?1
           AND c.concept_name NOT GLOB '*[.]*'
           AND length(c.concept_name) >= 3
           AND c.concept_name NOT GLOB '[0-9]*'
         ORDER BY occurrences DESC, c.concept_name ASC",
    )?;

    let nodes = concept_stmt
        .query_map(params![community_id], |row| {
            Ok(ConceptNode {
                name: row.get(0)?,
                occurrences: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let concept_set: HashSet<String> = nodes.iter().map(|node| node.name.clone()).collect();

    let mut edge_stmt = conn.prepare(
        "SELECT cg.concept_a, cg.concept_b, cg.co_occurrence_count
         FROM concept_graph cg
         JOIN concept_communities a_cc ON cg.concept_a = a_cc.concept_name
         JOIN concept_communities b_cc ON cg.concept_b = b_cc.concept_name
         WHERE a_cc.community_id = ?1 AND b_cc.community_id = ?1
         ORDER BY cg.co_occurrence_count DESC, cg.concept_a ASC, cg.concept_b ASC",
    )?;

    let edges = edge_stmt
        .query_map(params![community_id], |row| {
            Ok(ConceptEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|edge| concept_set.contains(&edge.source) && concept_set.contains(&edge.target))
        .collect();

    Ok(ConceptGraph {
        community_id,
        anchor,
        nodes,
        edges,
    })
}
