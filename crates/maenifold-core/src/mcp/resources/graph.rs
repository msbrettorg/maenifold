use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use crate::api;
use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::graph::{load_graph_neighbors, preview};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};

use super::throttle::TrailingThrottle;

pub const CORPUS_SNAPSHOT_URI: &str = "graph://corpus/snapshot";
pub const CORPUS_COMMUNITIES_URI: &str = "graph://corpus/communities";
pub const GRAPH_STATS_URI: &str = "graph://stats";
pub const CONCEPT_TEMPLATE_URI: &str = "graph://concept/{name}";
const CONCEPT_URI_PREFIX: &str = "graph://concept/";

static STATS_NOTIFICATION_THROTTLE: OnceLock<TrailingThrottle> = OnceLock::new();

#[derive(Debug, Serialize)]
struct CorpusSnapshot {
    nodes: Vec<SnapshotNode>,
    edges: Vec<api::graph::GraphEdge>,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct SnapshotNode {
    name: String,
    weight: i64,
}

#[derive(Debug, Serialize)]
struct CorpusCommunities {
    communities: Vec<Community>,
    edges: Vec<api::graph::GraphEdge>,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct Community {
    id: i64,
    members: Vec<String>,
    size: usize,
}

#[derive(Debug, Serialize)]
struct GraphStats {
    node_count: i64,
    edge_count: i64,
    community_count: i64,
    signals_per_sec: f64,
    last_sync_at: Option<String>,
    last_cluster_at: Option<String>,
}

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        make_resource(
            CORPUS_SNAPSHOT_URI,
            "Corpus Snapshot",
            "Full graph snapshot of concept nodes and edges.",
        ),
        make_resource(
            CORPUS_COMMUNITIES_URI,
            "Corpus Communities",
            "Detected graph communities with members and graph edges.",
        ),
        make_resource(
            GRAPH_STATS_URI,
            "Graph Stats",
            "Graph-wide statistics and sync/cluster timestamps.",
        ),
    ]
}

pub(super) fn list_resource_templates() -> Vec<rmcp::model::ResourceTemplate> {
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

pub(super) fn is_graph_uri(uri: &str) -> bool {
    uri.starts_with("graph://")
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let config = Config::current();
    let connection = db::open_connection(&config.database_path)?;

    let body = match uri {
        CORPUS_SNAPSHOT_URI => serde_json::to_string_pretty(&read_corpus_snapshot(&connection)?)?,
        CORPUS_COMMUNITIES_URI => {
            serde_json::to_string_pretty(&read_corpus_communities(&connection)?)?
        }
        GRAPH_STATS_URI => serde_json::to_string_pretty(&read_graph_stats(&connection)?)?,
        _ if uri.starts_with(CONCEPT_URI_PREFIX) => {
            serde_json::to_string_pretty(&read_concept_context(&connection, uri)?)?
        }
        _ => {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected one of '{CORPUS_SNAPSHOT_URI}', '{CORPUS_COMMUNITIES_URI}', '{GRAPH_STATS_URI}', or '{CONCEPT_TEMPLATE_URI}'"
            )));
        }
    };

    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        body,
        uri.to_string(),
    )
    .with_mime_type("application/json")]))
}

fn read_concept_context(connection: &Connection, uri: &str) -> Result<Value> {
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
    let canonical_name = crate::markdown::normalize_concept(decoded_name.trim());
    if canonical_name.is_empty() {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Missing concept name segment"
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
    let mut files = std::collections::BTreeSet::new();
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

pub async fn should_dispatch_stats_notification() -> bool {
    stats_notification_throttle().signal().await
}

pub async fn take_coalesced_stats_notification() -> bool {
    stats_notification_throttle().take_trailing_dispatch().await
}

fn stats_notification_throttle() -> &'static TrailingThrottle {
    STATS_NOTIFICATION_THROTTLE.get_or_init(|| TrailingThrottle::new(Duration::from_secs(1)))
}

fn make_resource(uri: &str, name: &str, description: &str) -> Resource {
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    resource.title = Some(name.to_string());
    resource.description = Some(description.to_string());
    resource
}

fn read_corpus_snapshot(connection: &Connection) -> Result<CorpusSnapshot> {
    let nodes = api::graph::get_nodes(connection, None, None, None)?
        .into_iter()
        .map(|node| SnapshotNode {
            name: node.name,
            weight: node.degree,
        })
        .collect();
    let edges = api::graph::get_edges(connection, None)?;

    Ok(CorpusSnapshot {
        nodes,
        edges,
        generated_at: generated_at(connection)?,
    })
}

fn read_corpus_communities(connection: &Connection) -> Result<CorpusCommunities> {
    let mut statement = connection.prepare(
        "SELECT community_id, concept_name
         FROM concept_communities
         ORDER BY community_id ASC, concept_name ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut grouped = BTreeMap::<i64, Vec<String>>::new();
    for row in rows {
        let (community_id, concept_name) = row?;
        grouped.entry(community_id).or_default().push(concept_name);
    }

    let communities = grouped
        .into_iter()
        .map(|(id, members)| Community {
            size: members.len(),
            id,
            members,
        })
        .collect();

    Ok(CorpusCommunities {
        communities,
        edges: api::graph::get_edges(connection, None)?,
        generated_at: generated_at(connection)?,
    })
}

fn read_graph_stats(connection: &Connection) -> Result<GraphStats> {
    let node_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM concepts", [], |row| row.get(0))?;
    let edge_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM concept_graph", [], |row| row.get(0))?;
    let community_count: i64 = connection.query_row(
        "SELECT COUNT(DISTINCT community_id) FROM concept_communities",
        [],
        |row| row.get(0),
    )?;

    let last_sync_at: Option<String> = connection
        .query_row(
            "SELECT MAX(last_indexed) FROM file_content WHERE last_indexed IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();

    let last_cluster_detected: Option<String> = connection
        .query_row(
            "SELECT MAX(detected_at) FROM concept_communities WHERE detected_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();

    let last_cluster_state: Option<String> = match connection.query_row(
        "SELECT value FROM incremental_sync_state WHERE key = 'last_community_recompute_at'",
        [],
        |row| row.get(0),
    ) {
        Ok(value) => Some(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(rusqlite::Error::SqliteFailure(_, _)) => None,
        Err(error) => return Err(error.into()),
    };

    Ok(GraphStats {
        node_count,
        edge_count,
        community_count,
        signals_per_sec: 0.0,
        last_sync_at,
        last_cluster_at: last_cluster_detected.or(last_cluster_state),
    })
}

fn generated_at(connection: &Connection) -> Result<String> {
    let from_file_content: Option<String> = connection
        .query_row(
            "SELECT MAX(last_indexed) FROM file_content WHERE last_indexed IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(from_file_content.unwrap_or_else(current_iso_timestamp))
}

fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}
