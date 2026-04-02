use crate::config::Config;
use crate::decay::get_decay_weight_for_file_with_connection;
use crate::error::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub(crate) struct GraphData {
    pub(crate) concepts: Vec<String>,
    pub(crate) adjacency: Vec<Vec<(usize, f64)>>,
    pub(crate) total_edge_weight: f64,
}

// T-12.0: RTM FR-23.6
pub(crate) fn load_graph(connection: &Connection, config: &Config) -> Result<GraphData> {
    let mut concepts_stmt =
        connection.prepare("SELECT concept_name FROM concepts ORDER BY concept_name")?;
    let concepts = concepts_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let index_by_name = concepts
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<HashMap<_, _>>();

    let mut adjacency = vec![Vec::new(); concepts.len()];
    let mut edge_stmt = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(source_files, '') \
         FROM concept_graph \
         ORDER BY concept_a, concept_b",
    )?;
    let edge_rows = edge_stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for edge_row in edge_rows {
        let (concept_a, concept_b, source_files) = edge_row?;
        let Some(&index_a) = index_by_name.get(&concept_a) else {
            continue;
        };
        let Some(&index_b) = index_by_name.get(&concept_b) else {
            continue;
        };

        let weight = graph_edge_weight(&source_files, connection, config);
        if weight <= 0.0 {
            continue;
        }
        adjacency[index_a].push((index_b, weight));
        adjacency[index_b].push((index_a, weight));
    }

    for neighbors in &mut adjacency {
        neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    }

    let total_edge_weight = adjacency
        .iter()
        .flat_map(|neighbors| neighbors.iter().map(|(_, weight)| *weight))
        .sum::<f64>()
        / 2.0;

    Ok(GraphData {
        concepts,
        adjacency,
        total_edge_weight,
    })
}

// T-15.8: RTM FR-13 — edge weight = sum(proximity × decay) matching C# RunLouvain.
// Each source file's proximity is multiplied by its temporal decay weight before summing.
pub(crate) fn graph_edge_weight(
    source_files: &str,
    connection: &Connection,
    config: &Config,
) -> f64 {
    let Ok(weights) = parse_source_file_weights(source_files) else {
        return 0.0;
    };
    weights
        .iter()
        .map(|(uri, proximity)| {
            let decay = get_decay_weight_for_file_with_connection(uri, connection, config);
            proximity * decay
        })
        .sum()
}

// T-4.3: RTM FR-23.x
pub(crate) fn parse_source_file_weights(source_files: &str) -> Result<BTreeMap<String, f64>> {
    let trimmed = source_files.trim();
    if trimmed.is_empty() {
        return Ok(BTreeMap::new());
    }

    match serde_json::from_str::<Value>(trimmed)? {
        Value::Object(map) => Ok(map
            .into_iter()
            .filter_map(|(path, value)| value.as_f64().map(|weight| (path, weight)))
            .collect()),
        Value::Array(entries) => Ok(entries
            .into_iter()
            .filter_map(|value| value.as_str().map(|path| (path.to_string(), 1.0)))
            .collect()),
        _ => Ok(BTreeMap::new()),
    }
}

