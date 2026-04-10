use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityNode {
    id: u32,
    anchor: String,
    size: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityEdge {
    source: u32,
    target: u32,
    weight: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityGraph {
    nodes: Vec<CommunityNode>,
    edges: Vec<CommunityEdge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptNode {
    name: String,
    occurrences: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptEdge {
    source: String,
    target: String,
    weight: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptGraph {
    community_id: u32,
    anchor: String,
    nodes: Vec<ConceptNode>,
    edges: Vec<ConceptEdge>,
}

fn resolve_db_path() -> Result<PathBuf, String> {
    let path = if let Ok(root) = std::env::var("MAENIFOLD_ROOT") {
        PathBuf::from(root).join("memory.db")
    } else {
        let home =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
        home.join("maenifold").join("memory.db")
    };
    if !path.exists() {
        return Err(format!("Graph database not found at {}", path.display()));
    }
    Ok(path)
}

fn open_readonly(path: &PathBuf) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open memory.db: {e}"))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Failed to set busy timeout: {e}"))?;
    Ok(conn)
}

#[tauri::command]
pub fn load_graph_communities() -> Result<CommunityGraph, String> {
    let db_path = resolve_db_path()?;
    let conn = open_readonly(&db_path)?;

    // Community nodes with decay-weighted anchor (from persistence.ts)
    let mut node_stmt = conn
        .prepare(
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
            ORDER BY cs.size DESC",
        )
        .map_err(|e| format!("Failed to prepare community query: {e}"))?;

    let nodes: Vec<CommunityNode> = node_stmt
        .query_map([], |row| {
            Ok(CommunityNode {
                id: row.get(0)?,
                anchor: row.get(1)?,
                size: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query communities: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode community rows: {e}"))?;

    let valid_ids: std::collections::HashSet<u32> = nodes.iter().map(|n| n.id).collect();

    // Cross-community edges (aggregated, filtered by weight threshold)
    let mut edge_stmt = conn
        .prepare(
            "SELECT a_cc.community_id, b_cc.community_id, SUM(cg.co_occurrence_count) as weight
             FROM concept_graph cg
             JOIN concept_communities a_cc ON cg.concept_a = a_cc.concept_name
             JOIN concept_communities b_cc ON cg.concept_b = b_cc.concept_name
             WHERE a_cc.community_id < b_cc.community_id
             GROUP BY a_cc.community_id, b_cc.community_id
             HAVING weight >= 5
             ORDER BY weight DESC
             LIMIT 2000",
        )
        .map_err(|e| format!("Failed to prepare edge query: {e}"))?;

    let edges: Vec<CommunityEdge> = edge_stmt
        .query_map([], |row| {
            Ok(CommunityEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query edges: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode edge rows: {e}"))?
        .into_iter()
        .filter(|e| valid_ids.contains(&e.source) && valid_ids.contains(&e.target))
        .collect();

    Ok(CommunityGraph { nodes, edges })
}

#[tauri::command]
pub fn load_community_concepts(community_id: u32) -> Result<ConceptGraph, String> {
    let db_path = resolve_db_path()?;
    let conn = open_readonly(&db_path)?;

    // Get anchor — decay-weighted (from persistence.ts)
    let anchor: String = conn
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
            ORDER BY raw_weight * max_decay DESC LIMIT 1",
            [community_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| format!("community-{community_id}"));

    // All concepts in this community
    let mut concept_stmt = conn
        .prepare(
            "SELECT c.concept_name, COALESCE(c.occurrence_count, 0) as occurrences
             FROM concepts c
             JOIN concept_communities cc ON c.concept_name = cc.concept_name
             WHERE cc.community_id = ?1
               AND c.concept_name NOT GLOB '*[.]*'
               AND length(c.concept_name) >= 3
               AND c.concept_name NOT GLOB '[0-9]*'
             ORDER BY occurrences DESC",
        )
        .map_err(|e| format!("Failed to prepare concept query: {e}"))?;

    let nodes: Vec<ConceptNode> = concept_stmt
        .query_map([community_id], |row| {
            Ok(ConceptNode {
                name: row.get(0)?,
                occurrences: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query concepts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode concept rows: {e}"))?;

    let concept_set: HashMap<String, bool> = nodes.iter().map(|n| (n.name.clone(), true)).collect();

    // Intra-community edges
    let mut edge_stmt = conn
        .prepare(
            "SELECT cg.concept_a, cg.concept_b, cg.co_occurrence_count
             FROM concept_graph cg
             JOIN concept_communities a_cc ON cg.concept_a = a_cc.concept_name
             JOIN concept_communities b_cc ON cg.concept_b = b_cc.concept_name
             WHERE a_cc.community_id = ?1 AND b_cc.community_id = ?1",
        )
        .map_err(|e| format!("Failed to prepare intra-community edge query: {e}"))?;

    let edges: Vec<ConceptEdge> = edge_stmt
        .query_map([community_id], |row| {
            Ok(ConceptEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query intra-community edges: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode intra-community edge rows: {e}"))?
        .into_iter()
        .filter(|e| concept_set.contains_key(&e.source) && concept_set.contains_key(&e.target))
        .collect();

    Ok(ConceptGraph {
        community_id,
        anchor,
        nodes,
        edges,
    })
}
