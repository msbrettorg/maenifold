use super::data::load_graph;
use super::louvain::{fallback_to_connected_components, run_louvain};
use super::metrics::maybe_warn_on_community_drift;
use crate::config::Config;
use crate::db;
use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// T-12.0: RTM FR-23.7
// T-15.10: RTM FR-23.7 — seed parameter for deterministic Louvain node ordering.
pub fn detect_and_persist(db_path: &Path, gamma: f64, seed: Option<u64>) -> Result<usize> {
    let mut connection = db::open_connection(db_path)?;
    let config = Config::current();
    let previous_assignments = load_assignments(&connection)?;
    let graph = load_graph(&connection, &config)?;
    let (communities, modularity) = run_louvain(&graph, gamma, seed);
    let (communities, modularity) =
        fallback_to_connected_components(&graph, communities, modularity, gamma);
    persist_assignments(
        &mut connection,
        &graph.concepts,
        &communities,
        modularity,
        gamma,
    )?;
    maybe_warn_on_community_drift(&previous_assignments, &graph.concepts, &communities);
    Ok(communities.iter().copied().collect::<BTreeSet<_>>().len())
}

// T-4.3: RTM FR-23.7
pub(crate) fn load_assignments(connection: &Connection) -> Result<BTreeMap<String, usize>> {
    let mut statement = connection.prepare(
        "SELECT concept_name, community_id
         FROM concept_communities
         ORDER BY concept_name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    })?;
    Ok(rows.collect::<std::result::Result<BTreeMap<_, _>, _>>()?)
}

// T-12.0: RTM FR-23.7
pub(crate) fn persist_assignments(
    connection: &mut Connection,
    concepts: &[String],
    communities: &[usize],
    modularity: f64,
    gamma: f64,
) -> Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM concept_communities", [])?;

    for (concept_name, community_id) in concepts.iter().zip(communities.iter().copied()) {
        transaction.execute(
            "INSERT INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![concept_name, community_id as i64, modularity, gamma],
        )?;
    }

    transaction.commit()?;
    Ok(())
}
