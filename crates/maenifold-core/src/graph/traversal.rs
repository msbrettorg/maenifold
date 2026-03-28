use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) struct GraphRelation {
    pub(crate) name: String,
    pub(crate) co_occurrence_count: f64,
    pub(crate) source_files: String,
}

pub(crate) struct RelatedConceptLine {
    pub(crate) name: String,
    pub(crate) co_occurrence_count: f64,
    pub(crate) source_files: String,
    pub(crate) community_id: Option<i64>,
}

pub(crate) struct CommunitySiblingLine {
    pub(crate) name: String,
    pub(crate) community_id: i64,
    pub(crate) normalized_overlap: f64,
}

pub(crate) fn load_graph_neighbors(
    connection: &Connection,
    concept_name: &str,
) -> Result<Vec<GraphRelation>> {
    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') \
         FROM concept_graph \
         WHERE concept_a = ?1 OR concept_b = ?1 \
         ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC",
    )?;
    let rows = statement.query_map([concept_name], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut relations = Vec::new();
    for row in rows {
        let (concept_a, concept_b, co_occurrence_count, source_files) = row?;
        let name = if concept_a == concept_name {
            concept_b
        } else {
            concept_a
        };
        if name == concept_name {
            continue;
        }
        relations.push(GraphRelation {
            name,
            co_occurrence_count,
            source_files,
        });
    }

    Ok(relations)
}

pub(crate) fn load_concept_community_id(
    connection: &Connection,
    concept_name: &str,
) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
            params![concept_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn load_community_siblings(
    connection: &Connection,
    concept_name: &str,
    community_id: i64,
    direct_relation_names: &HashSet<String>,
) -> Result<Vec<CommunitySiblingLine>> {
    let query_neighbors = load_neighbor_weights(connection, concept_name)?;
    if query_neighbors.is_empty() {
        return Ok(Vec::new());
    }

    let min_shared_neighbors = std::env::var("MAENIFOLD_COMMUNITY_MIN_SHARED")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let min_overlap = std::env::var("MAENIFOLD_COMMUNITY_MIN_OVERLAP")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.4);
    let max_siblings = std::env::var("MAENIFOLD_COMMUNITY_MAX_SIBLINGS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);
    let candidate_names =
        load_community_sibling_candidates(connection, concept_name, community_id)?;

    let mut siblings = Vec::new();
    for sibling_name in candidate_names {
        if direct_relation_names.contains(&sibling_name) {
            continue;
        }

        let sibling_neighbors = load_neighbor_weights(connection, &sibling_name)?;
        if sibling_neighbors.is_empty() {
            continue;
        }

        let sibling_total_degree = sibling_neighbors.values().sum::<f64>();
        if sibling_total_degree <= 0.0 {
            continue;
        }

        let mut shared_neighbor_count = 0usize;
        let mut overlap_numerator = 0.0;
        for (neighbor, query_weight) in &query_neighbors {
            let Some(sibling_weight) = sibling_neighbors.get(neighbor) else {
                continue;
            };
            shared_neighbor_count += 1;
            overlap_numerator += (*query_weight).min(*sibling_weight);
        }

        let normalized_overlap = overlap_numerator / sibling_total_degree;
        if shared_neighbor_count < min_shared_neighbors || normalized_overlap < min_overlap {
            continue;
        }

        siblings.push((
            CommunitySiblingLine {
                community_id: load_concept_community_id(connection, &sibling_name)?
                    .unwrap_or(community_id),
                name: sibling_name.clone(),
                normalized_overlap,
            },
            shared_neighbor_count,
        ));
    }

    siblings.sort_by(|left, right| {
        right
            .0
            .normalized_overlap
            .partial_cmp(&left.0.normalized_overlap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.0.name.cmp(&right.0.name))
    });

    Ok(siblings
        .into_iter()
        .take(max_siblings)
        .map(|(sibling, _)| sibling)
        .collect())
}

fn load_neighbor_weights(
    connection: &Connection,
    concept_name: &str,
) -> Result<HashMap<String, f64>> {
    Ok(load_graph_neighbors(connection, concept_name)?
        .into_iter()
        .map(|relation| (relation.name, relation.co_occurrence_count))
        .collect())
}

// T-15.7: RTM FR-13.7 — restrict sibling candidates to same community only.
// C# only queries concept_communities for the matching community_id; no all-concepts fallback.
fn load_community_sibling_candidates(
    connection: &Connection,
    concept_name: &str,
    community_id: i64,
) -> Result<Vec<String>> {
    load_candidate_names(
        connection,
        "SELECT concept_name \
         FROM concept_communities \
         WHERE community_id = ?1 AND concept_name != ?2 \
         ORDER BY concept_name ASC",
        params![community_id, concept_name],
    )
}

fn load_candidate_names<P>(connection: &Connection, sql: &str, params: P) -> Result<Vec<String>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params, |row| row.get::<_, String>(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}
