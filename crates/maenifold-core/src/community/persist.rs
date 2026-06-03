use super::data::load_graph;
use super::events::{self, CommunityEvent};
use super::louvain::{fallback_to_connected_components, run_louvain};
use super::metrics::{compute_nmi, maybe_warn_on_community_drift};
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
    let current_assignments = graph
        .concepts
        .iter()
        .cloned()
        .zip(communities.iter().copied())
        .collect::<BTreeMap<_, _>>();
    let nmi = compute_nmi(&previous_assignments, &current_assignments);
    let community_count = communities.iter().copied().collect::<BTreeSet<_>>().len();
    let _ = events::sender().send(CommunityEvent::Recomputed {
        community_count,
        nmi,
    });
    Ok(community_count)
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::Utc;
    use rusqlite::params;

    use super::*;
    use crate::acp_config::MaenifoldSettings;
    use crate::db::GraphDatabase;

    #[test]
    #[serial_test::serial]
    fn detect_and_persist_publishes_recomputed_event() -> Result<()> {
        let root = tempfile::tempdir()?;
        let config = Config {
            maenifold_root: root.path().to_path_buf(),
            memory_path: root.path().join("memory"),
            assets_path: root.path().join("assets"),
            thinking_path: root.path().join("memory").join("thinking"),
            sequential_path: root
                .path()
                .join("memory")
                .join("thinking")
                .join("sequential"),
            database_path: root.path().join("memory.db"),
            settings: MaenifoldSettings::default(),
        };

        GraphDatabase::initialize_database(&config)?;
        Config::override_root(&config.maenifold_root);
        Config::set_database_path(&config.database_path);

        let result = (|| -> Result<()> {
            seed_two_concepts(&config)?;
            let mut events = events::subscribe();

            let community_count = detect_and_persist(&config.database_path, 1.0, Some(7))?;
            let event = events.try_recv()?;

            assert_eq!(community_count, 1);
            assert_eq!(
                event,
                CommunityEvent::Recomputed {
                    community_count,
                    nmi: 0.0
                }
            );
            Ok(())
        })();

        Config::reset_overrides();
        result
    }

    fn seed_two_concepts(config: &Config) -> Result<()> {
        let conn = db::open_connection(&config.database_path)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO file_content (file_path, title, content, last_indexed, created, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?4, ?4)",
            params![
                "memory://community-event-test.md",
                "Community Event Test",
                "[[alpha]] [[beta]]",
                now
            ],
        )?;

        for (concept, community_id) in [("alpha", 1_i64), ("beta", 2_i64)] {
            conn.execute(
                "INSERT INTO concepts (concept_name, first_seen, occurrence_count)
                 VALUES (?1, ?2, 1)",
                params![concept, now],
            )?;
            conn.execute(
                "INSERT INTO concept_communities
                    (concept_name, community_id, modularity, resolution, detected_at)
                 VALUES (?1, ?2, 0.0, 1.0, ?3)",
                params![concept, community_id, now],
            )?;
        }

        let source_files =
            serde_json::json!({ "memory://community-event-test.md": 1.0 }).to_string();
        conn.execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
             VALUES ('alpha', 'beta', 1, ?1)",
            params![source_files],
        )?;
        Ok(())
    }
}
