use std::collections::BTreeSet;

use crate::error::{MaenifoldError, Result};
use crate::graph::{load_graph_neighbors, preview};
use rmcp::model::{AnnotateAble, ResourceTemplate};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

pub(super) const CONCEPT_TEMPLATE_URI: &str = "graph://concept/{name}";
const CONCEPT_URI_PREFIX: &str = "graph://concept/";

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![rmcp::model::RawResourceTemplate {
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
    .optional_annotate(None)]
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

    let concept_exists: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM concepts WHERE concept_name = ?1 LIMIT 1",
            [canonical_name.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    if concept_exists.is_none() {
        return Err(MaenifoldError::AssetNotFound(
            "concept".to_string(),
            canonical_name,
        ));
    }

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

    Ok(json!({
        "concept": canonical_name,
        "coOccurrenceCount": co_occurrence_count,
        "neighbors": neighbors,
        "files": files.into_iter().collect::<Vec<_>>(),
    }))
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
