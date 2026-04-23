use std::collections::BTreeMap;

use crate::api;
use crate::error::{MaenifoldError, Result};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use super::graph_concept;

pub(super) const CORPUS_SNAPSHOT_URI: &str = "graph://corpus/snapshot";
pub(super) const CORPUS_COMMUNITIES_URI: &str = "graph://corpus/communities";
pub(super) const GRAPH_STATS_URI: &str = "graph://stats";

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
    last_sync_at: Option<String>,
    last_cluster_at: Option<String>,
}

pub(super) enum GraphReadRouteError {
    NotHandled,
    ResourceError(MaenifoldError),
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

pub(super) fn read_or_route_to_concept(
    uri: &str,
    connection: &Connection,
) -> std::result::Result<ReadResourceResult, GraphReadRouteError> {
    let body = match uri {
        CORPUS_SNAPSHOT_URI => serde_json::to_string_pretty(
            &read_corpus_snapshot(connection).map_err(GraphReadRouteError::ResourceError)?,
        )
        .map_err(|err| GraphReadRouteError::ResourceError(err.into()))?,
        CORPUS_COMMUNITIES_URI => serde_json::to_string_pretty(
            &read_corpus_communities(connection).map_err(GraphReadRouteError::ResourceError)?,
        )
        .map_err(|err| GraphReadRouteError::ResourceError(err.into()))?,
        GRAPH_STATS_URI => serde_json::to_string_pretty(
            &read_graph_stats(connection).map_err(GraphReadRouteError::ResourceError)?,
        )
        .map_err(|err| GraphReadRouteError::ResourceError(err.into()))?,
        _ => {
            let concept_payload = graph_concept::read_concept_context(connection, uri).map_err(
                |error| match error {
                    MaenifoldError::InvalidResourceUri(_) => GraphReadRouteError::NotHandled,
                    other => GraphReadRouteError::ResourceError(other),
                },
            )?;
            serde_json::to_string_pretty(&concept_payload)
                .map_err(|err| GraphReadRouteError::ResourceError(err.into()))?
        }
    };

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(body, uri.to_string()).with_mime_type("application/json"),
    ]))
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
