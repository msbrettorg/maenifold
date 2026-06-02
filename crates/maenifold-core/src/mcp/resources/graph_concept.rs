use std::collections::BTreeSet;

use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::graph::{load_graph_neighbors, preview, relation_weighted_score};
use rmcp::model::{AnnotateAble, ResourceTemplate};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

pub(super) const CONCEPT_TEMPLATE_URI: &str = "graph://concept/{name}";
pub(super) const CONCEPT_NEIGHBORS_TEMPLATE_URI: &str = "graph://concept/{name}/neighbors";
const CONCEPT_URI_PREFIX: &str = "graph://concept/";
const CONCEPT_NEIGHBORS_SUFFIX: &str = "/neighbors";
const DEFAULT_NEIGHBOR_LIMIT: usize = 50;
const MAX_NEIGHBOR_LIMIT: usize = 200;

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        rmcp::model::RawResourceTemplate {
            uri_template: CONCEPT_TEMPLATE_URI.to_string(),
            name: "Graph Concept Context".to_string(),
            title: Some("Graph Concept Context".to_string()),
            description: Some(
                "Per-concept graph context with neighbors, co-occurrence counts, and source files."
                    .to_string(),
            ),
            icons: None,
            mime_type: Some("application/json".to_string()),
        }
        .optional_annotate(None),
        rmcp::model::RawResourceTemplate {
            uri_template: CONCEPT_NEIGHBORS_TEMPLATE_URI.to_string(),
            name: "Graph Concept Neighbors".to_string(),
            title: Some("Graph Concept Neighbors".to_string()),
            description: Some(
                "Paged per-concept graph neighbors for bounded knowledge navigation.".to_string(),
            ),
            icons: None,
            mime_type: Some("application/json".to_string()),
        }
        .optional_annotate(None),
    ]
}

pub(super) fn read_concept_neighbors(connection: &Connection, uri: &str) -> Result<Option<Value>> {
    let Some(request) = parse_neighbors_uri(uri)? else {
        return Ok(None);
    };
    ensure_concept_exists(connection, &request.concept)?;

    let total_count = count_neighbors(connection, &request.concept)?;
    let rows = load_neighbor_page(connection, &request)?;
    let config = Config::current();
    let mut neighbors = Vec::with_capacity(rows.len());

    for relation in rows {
        let files = parse_relation_source_files(&relation.source_files);
        let (decay_weight, weighted_score) = relation_weighted_score(
            connection,
            &config,
            relation.co_occurrence_count,
            &relation.source_files,
        )?;
        let community_id: Option<i64> = connection
            .query_row(
                "SELECT community_id FROM concept_communities WHERE concept_name = ?1 LIMIT 1",
                [relation.name.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        neighbors.push(json!({
            "name": relation.name,
            "coOccurrenceCount": relation.co_occurrence_count,
            "decayWeight": decay_weight,
            "weightedScore": weighted_score,
            "communityId": community_id,
            "sourceFileCount": files.len(),
            "files": files
        }));
    }

    let next_cursor = request
        .cursor
        .checked_add(neighbors.len())
        .filter(|next| (*next as i64) < total_count)
        .map(|next| next.to_string());

    Ok(Some(json!({
        "concept": request.concept,
        "communityId": load_concept_community_id(connection, &request.concept)?,
        "limit": request.limit,
        "cursor": request.cursor.to_string(),
        "nextCursor": next_cursor,
        "totalCount": total_count,
        "order": "coOccurrenceCountDesc",
        "neighbors": neighbors
    })))
}

pub(super) fn read_concept_context(connection: &Connection, uri: &str) -> Result<Value> {
    let encoded_name = uri.strip_prefix(CONCEPT_URI_PREFIX).ok_or_else(|| {
        MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected URI to begin with '{CONCEPT_URI_PREFIX}'"
        ))
    })?;
    let decoded_name = urlencoding::decode(encoded_name)
        .map_err(|e| {
            MaenifoldError::InvalidResourceUri(format!(
                "Failed to percent-decode concept segment '{encoded_name}': {e}"
            ))
        })?
        .into_owned();
    if decoded_name.trim().is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. concept name is required"
        )));
    }
    let canonical_name = crate::markdown::normalize_concept(decoded_name.trim());
    if canonical_name.is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. concept name is required"
        )));
    }

    ensure_concept_exists(connection, &canonical_name)?;

    let relations = load_graph_neighbors(connection, &canonical_name)?;
    let mut files = BTreeSet::new();
    let neighbors = relations
        .into_iter()
        .map(|relation| {
            let neighbor_files = parse_relation_source_files(&relation.source_files);
            files.extend(neighbor_files.iter().cloned());
            json!({
                "name": relation.name,
                "coOccurrenceCount": relation.co_occurrence_count,
                "files": neighbor_files
            })
        })
        .collect::<Vec<_>>();
    let co_occurrence_count = neighbors
        .iter()
        .filter_map(|neighbor| neighbor.get("coOccurrenceCount").and_then(Value::as_f64))
        .sum::<f64>();
    let community_id: Option<i64> = connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1 LIMIT 1",
            [canonical_name.as_str()],
            |row| row.get(0),
        )
        .optional()?;

    Ok(json!({
        "concept": canonical_name,
        "communityId": community_id,
        "coOccurrenceCount": co_occurrence_count,
        "neighbors": neighbors,
        "files": files.into_iter().collect::<Vec<_>>(),
    }))
}

struct NeighborPageRequest {
    concept: String,
    limit: usize,
    cursor: usize,
}

fn parse_neighbors_uri(uri: &str) -> Result<Option<NeighborPageRequest>> {
    let Some(rest) = uri.strip_prefix(CONCEPT_URI_PREFIX) else {
        return Ok(None);
    };
    let (path, query) = rest
        .split_once('?')
        .map_or((rest, ""), |(path, query)| (path, query));
    let Some(encoded_name) = path.strip_suffix(CONCEPT_NEIGHBORS_SUFFIX) else {
        return Ok(None);
    };
    let concept = decode_concept_segment(uri, encoded_name)?;
    let (limit, cursor) = parse_neighbor_query(uri, query)?;
    Ok(Some(NeighborPageRequest {
        concept,
        limit,
        cursor,
    }))
}

fn decode_concept_segment(uri: &str, encoded_name: &str) -> Result<String> {
    let decoded_name = urlencoding::decode(encoded_name)
        .map_err(|e| {
            MaenifoldError::InvalidResourceUri(format!(
                "Failed to percent-decode concept segment '{encoded_name}': {e}"
            ))
        })?
        .into_owned();
    if decoded_name.trim().is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. concept name is required"
        )));
    }
    let canonical_name = crate::markdown::normalize_concept(decoded_name.trim());
    if canonical_name.is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. concept name is required"
        )));
    }
    Ok(canonical_name)
}

fn parse_neighbor_query(uri: &str, query: &str) -> Result<(usize, usize)> {
    let mut limit = DEFAULT_NEIGHBOR_LIMIT;
    let mut cursor = 0usize;
    for pair in query.split('&').filter(|segment| !segment.is_empty()) {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Invalid query parameter '{pair}'"))
        })?;
        match key {
            "limit" => limit = parse_positive_usize(uri, key, value)?,
            "cursor" => cursor = parse_usize(uri, key, value)?,
            other => {
                return Err(MaenifoldError::InvalidResourceUri(format!(
                    "{uri}. Unsupported query parameter '{other}'"
                )));
            }
        }
    }
    if limit > MAX_NEIGHBOR_LIMIT {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. limit must be <= {MAX_NEIGHBOR_LIMIT}"
        )));
    }
    Ok((limit, cursor))
}

fn parse_positive_usize(uri: &str, key: &str, value: &str) -> Result<usize> {
    let parsed = parse_usize(uri, key, value)?;
    if parsed == 0 {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. {key} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_usize(uri: &str, key: &str, value: &str) -> Result<usize> {
    value.parse::<usize>().map_err(|error| {
        MaenifoldError::InvalidResourceUri(format!("{uri}. Invalid {key} '{value}': {error}"))
    })
}

fn ensure_concept_exists(connection: &Connection, canonical_name: &str) -> Result<()> {
    let concept_exists: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM concepts WHERE concept_name = ?1 LIMIT 1",
            [canonical_name],
            |row| row.get(0),
        )
        .optional()?;
    if concept_exists.is_none() {
        return Err(MaenifoldError::AssetNotFound(
            "concept".to_string(),
            canonical_name.to_string(),
        ));
    }
    Ok(())
}

fn load_concept_community_id(connection: &Connection, concept_name: &str) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1 LIMIT 1",
            [concept_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn count_neighbors(connection: &Connection, concept_name: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1",
            [concept_name],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn load_neighbor_page(
    connection: &Connection,
    request: &NeighborPageRequest,
) -> Result<Vec<crate::graph::GraphRelation>> {
    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') \
         FROM concept_graph \
         WHERE concept_a = ?1 OR concept_b = ?1 \
         ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC \
         LIMIT ?2 OFFSET ?3",
    )?;
    let rows = statement.query_map(
        params![request.concept, request.limit as i64, request.cursor as i64],
        |row| {
            let concept_a = row.get::<_, String>(0)?;
            let concept_b = row.get::<_, String>(1)?;
            let name = if concept_a == request.concept {
                concept_b
            } else {
                concept_a
            };
            Ok(crate::graph::GraphRelation {
                name,
                co_occurrence_count: row.get(2)?,
                source_files: row.get(3)?,
            })
        },
    )?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn parse_relation_source_files(source_files: &str) -> Vec<String> {
    match preview::parse_source_files_for_json(source_files) {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect(),
        Value::String(single) if !single.trim().is_empty() => vec![single],
        _ => Vec::new(),
    }
}
