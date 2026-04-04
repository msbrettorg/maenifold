use crate::error::Result;
use crate::markdown::normalize_concept;
use rusqlite::{Connection, params_from_iter};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub struct EndophasiaCommunitiesResponse {
    pub communities: Vec<EndophasiaCommunity>,
    pub edges: Vec<EndophasiaEdge>,
}

#[derive(Debug, Serialize)]
pub struct EndophasiaCommunity {
    pub id: i64,
    pub concepts: Vec<String>,
    pub anchor_score: f64,
}

#[derive(Debug, Serialize)]
pub struct EndophasiaEdge {
    pub source: String,
    pub target: String,
    pub weight: i64,
}

struct CommunityRow {
    community_id: i64,
    concept_name: String,
    _rank: i64,
    anchor_score: f64,
}

pub fn load_response(
    connection: &Connection,
    params: &HashMap<String, String>,
) -> Result<EndophasiaCommunitiesResponse> {
    if !has_any_communities(connection)? {
        return Ok(EndophasiaCommunitiesResponse {
            communities: Vec::new(),
            edges: Vec::new(),
        });
    }

    let context_concepts = parse_context_concepts(params.get("context"));
    let matching_community_ids = if context_concepts.is_empty() {
        None
    } else {
        let ids = load_matching_community_ids(connection, &context_concepts)?;
        if ids.is_empty() {
            return Ok(EndophasiaCommunitiesResponse {
                communities: Vec::new(),
                edges: Vec::new(),
            });
        }
        Some(ids)
    };

    let community_rows = load_community_rows(connection, matching_community_ids.as_deref())?;
    if community_rows.is_empty() {
        return Ok(EndophasiaCommunitiesResponse {
            communities: Vec::new(),
            edges: Vec::new(),
        });
    }

    let communities = build_communities(community_rows);
    let concept_names = communities
        .iter()
        .flat_map(|community| community.concepts.iter().cloned())
        .collect::<Vec<_>>();
    let edges = load_edges(connection, &concept_names)?;

    Ok(EndophasiaCommunitiesResponse { communities, edges })
}

fn has_any_communities(connection: &Connection) -> Result<bool> {
    let mut statement = connection.prepare("SELECT 1 FROM concept_communities LIMIT 1")?;
    Ok(statement.exists([])?)
}

fn parse_context_concepts(raw: Option<&String>) -> Vec<String> {
    let mut seen = HashSet::new();
    raw.into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_concept)
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn load_matching_community_ids(
    connection: &Connection,
    context_concepts: &[String],
) -> Result<Vec<i64>> {
    let placeholders = repeat_placeholders(context_concepts.len());
    let sql = format!(
        "SELECT DISTINCT community_id \
         FROM concept_communities \
         WHERE concept_name IN ({placeholders}) \
         ORDER BY community_id ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(context_concepts.iter()), |row| row.get(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_community_rows(
    connection: &Connection,
    matching_community_ids: Option<&[i64]>,
) -> Result<Vec<CommunityRow>> {
    let mut sql = String::from(
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
                            ELSE MIN(
                                1.0,
                                1.0 / sqrt(MAX(
                                    1,
                                    julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) -
                                    CASE
                                        WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                                        WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                                        WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                                        ELSE 28
                                    END
                                ))
                            )
                        END
                    ) AS max_decay
             FROM concept_mentions cm
             JOIN file_content fc ON cm.source_file = fc.file_path
             WHERE COALESCE(fc.last_accessed, fc.created, fc.last_indexed) IS NOT NULL
             GROUP BY cm.concept_name
         ),
         community_sizes AS (
             SELECT community_id, COUNT(*) AS size
             FROM concept_communities
             GROUP BY community_id
             HAVING size >= 3
         ),
         concept_degree AS (
             SELECT cc.concept_name,
                    cc.community_id,
                    SUM(cg.co_occurrence_count) AS raw_weight,
                    COALESCE(cd.max_decay, 0.1) AS max_decay
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
             SELECT *,
                    raw_weight * max_decay AS weighted_score,
                    ROW_NUMBER() OVER (
                        PARTITION BY community_id
                        ORDER BY raw_weight * max_decay DESC, concept_name ASC
                    ) AS rn
             FROM concept_degree
             WHERE raw_weight > 0
         ),
         anchors AS (
             SELECT community_id,
                    concept_name AS anchor,
                    weighted_score AS anchor_score
             FROM ranked
             WHERE rn = 1
         )
         SELECT r.community_id,
                r.concept_name,
                r.rn,
                a.anchor_score
         FROM ranked r
         JOIN anchors a ON a.community_id = r.community_id
         WHERE r.rn <= 10",
    );

    if let Some(ids) = matching_community_ids {
        sql.push_str(&format!(
            " AND r.community_id IN ({})",
            repeat_placeholders(ids.len())
        ));
    }

    sql.push_str(" ORDER BY a.anchor_score DESC, r.community_id ASC, r.rn ASC, r.concept_name ASC");

    let mut statement = connection.prepare(&sql)?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(CommunityRow {
            community_id: row.get(0)?,
            concept_name: row.get(1)?,
            _rank: row.get::<_, i64>(2)?,
            anchor_score: row.get(3)?,
        })
    };

    let rows = if let Some(ids) = matching_community_ids {
        statement.query_map(params_from_iter(ids.iter()), mapper)?
    } else {
        statement.query_map([], mapper)?
    };

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn build_communities(rows: Vec<CommunityRow>) -> Vec<EndophasiaCommunity> {
    let mut communities: Vec<EndophasiaCommunity> = Vec::new();

    for row in rows {
        match communities.last_mut() {
            Some(community) if community.id == row.community_id => {
                community.concepts.push(row.concept_name);
            }
            _ => {
                if communities.len() == 10 {
                    break;
                }
                communities.push(EndophasiaCommunity {
                    id: row.community_id,
                    concepts: vec![row.concept_name],
                    anchor_score: row.anchor_score,
                });
            }
        }
    }

    communities
}

fn load_edges(connection: &Connection, concept_names: &[String]) -> Result<Vec<EndophasiaEdge>> {
    if concept_names.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repeat_placeholders(concept_names.len());
    let sql = format!(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0) \
         FROM concept_graph \
         WHERE concept_a IN ({placeholders}) AND concept_b IN ({placeholders}) \
         ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC"
    );

    let params = concept_names
        .iter()
        .chain(concept_names.iter())
        .collect::<Vec<_>>();
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params.into_iter()), |row| {
        Ok(EndophasiaEdge {
            source: row.get(0)?,
            target: row.get(1)?,
            weight: row.get(2)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn repeat_placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}
