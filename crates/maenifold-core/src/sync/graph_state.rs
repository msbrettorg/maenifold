use super::graph_relations::{
    RelationCounts, load_relation_counts, load_relation_counts_for_files,
};
use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn rebuild_graph_from_mentions(
    connection: &Connection,
    indexed_at: &str,
) -> Result<i64> {
    let existing_first_seen = load_existing_first_seen(connection)?;
    let mut statement = connection.prepare(
        "SELECT source_file, concept_name, mention_count FROM concept_mentions ORDER BY source_file, concept_name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut total_mentions = 0i64;
    let mut concept_counts = BTreeMap::<String, i64>::new();
    for row in rows {
        let (_source_file, concept_name, mention_count) = row?;
        total_mentions += mention_count;
        *concept_counts.entry(concept_name.clone()).or_default() += mention_count;
    }

    let relation_counts = load_relation_counts(connection)?;
    connection.execute("DELETE FROM concept_graph", [])?;

    for (concept_name, occurrence_count) in concept_counts {
        let first_seen = existing_first_seen
            .get(&concept_name)
            .cloned()
            .unwrap_or_else(|| indexed_at.to_string());
        connection.execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)
             ON CONFLICT(concept_name) DO UPDATE SET
                 first_seen = COALESCE(concepts.first_seen, excluded.first_seen),
                 occurrence_count = excluded.occurrence_count",
            params![concept_name, first_seen, occurrence_count],
        )?;
    }

    purge_unreferenced_concepts(connection)?;
    write_relation_counts(connection, relation_counts)?;
    Ok(total_mentions)
}

// T-13.6: RTM NFR-1 — incremental graph rebuild for affected concepts during sync
pub(crate) fn rebuild_affected_graph_from_mentions(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
    indexed_at: &str,
) -> Result<usize> {
    if affected_concepts.is_empty() {
        return Ok(0);
    }

    let existing_first_seen = load_existing_first_seen(connection)?;
    let mut existing_relations =
        load_existing_relations_for_concepts(connection, affected_concepts)?;
    refresh_affected_concepts(
        connection,
        affected_concepts,
        &existing_first_seen,
        indexed_at,
    )?;

    let candidate_files = load_candidate_files_for_concepts(connection, affected_concepts)?;
    let relation_counts =
        load_relation_counts_for_files(connection, &candidate_files, Some(affected_concepts))?;

    let mut relation_count = 0usize;
    for ((concept_a, concept_b), (count, files)) in relation_counts {
        let key = (concept_a.clone(), concept_b.clone());
        let source_files = serde_json::to_string(&files)?;
        let should_write = existing_relations.remove(&key).is_none_or(|existing| {
            existing.co_occurrence_count != count || existing.source_files != source_files
        });
        if should_write {
            connection.execute(
                "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(concept_a, concept_b) DO UPDATE SET
                     co_occurrence_count = excluded.co_occurrence_count,
                     source_files = excluded.source_files",
                params![concept_a, concept_b, count, source_files],
            )?;
        }
        relation_count += 1;
    }

    for ((concept_a, concept_b), _) in existing_relations {
        connection.execute(
            "DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![concept_a, concept_b],
        )?;
    }

    Ok(relation_count)
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingRelationRow {
    pub(crate) co_occurrence_count: f64,
    pub(crate) source_files: String,
}

pub(crate) fn load_existing_first_seen(
    connection: &Connection,
) -> Result<BTreeMap<String, String>> {
    let mut statement = connection
        .prepare("SELECT concept_name, first_seen FROM concepts WHERE first_seen IS NOT NULL")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<std::result::Result<BTreeMap<_, _>, _>>()?)
}

pub(crate) fn load_existing_relations_for_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<BTreeMap<(String, String), ExistingRelationRow>> {
    let mut relations = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '')
         FROM concept_graph
         WHERE concept_a = ?1 OR concept_b = ?1",
    )?;
    for concept in affected_concepts {
        let rows = statement.query_map(params![concept], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                ExistingRelationRow {
                    co_occurrence_count: row.get::<_, f64>(2)?,
                    source_files: row.get::<_, String>(3)?,
                },
            ))
        })?;
        for row in rows {
            let (concept_a, concept_b, relation) = row?;
            relations.entry((concept_a, concept_b)).or_insert(relation);
        }
    }
    Ok(relations)
}

// T-13.6: RTM NFR-1 — refreshes concept occurrence counts for affected concepts
pub(crate) fn refresh_affected_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
    existing_first_seen: &BTreeMap<String, String>,
    indexed_at: &str,
) -> Result<()> {
    let mut mention_statement = connection.prepare(
        "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE concept_name = ?1",
    )?;
    let mut upsert_statement = connection.prepare(
        "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)
         ON CONFLICT(concept_name) DO UPDATE SET occurrence_count = excluded.occurrence_count, first_seen = COALESCE(concepts.first_seen, excluded.first_seen)",
    )?;
    let mut delete_concept_statement =
        connection.prepare("DELETE FROM concepts WHERE concept_name = ?1")?;
    let mut delete_community_statement =
        connection.prepare("DELETE FROM concept_communities WHERE concept_name = ?1")?;

    for concept in affected_concepts {
        let occurrence_count: i64 =
            mention_statement.query_row(params![concept], |row| row.get(0))?;
        if occurrence_count <= 0 {
            delete_community_statement.execute(params![concept])?;
            delete_concept_statement.execute(params![concept])?;
            continue;
        }

        let first_seen = existing_first_seen
            .get(concept)
            .cloned()
            .unwrap_or_else(|| indexed_at.to_string());
        upsert_statement.execute(params![concept, first_seen, occurrence_count])?;
    }

    Ok(())
}

pub(crate) fn prune_orphaned_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<()> {
    let mut delete_concept_statement = connection.prepare(
        "DELETE FROM concepts
         WHERE concept_name = ?1
           AND NOT EXISTS (
               SELECT 1 FROM concept_mentions WHERE concept_name = ?1
           )",
    )?;
    let mut delete_community_statement = connection.prepare(
        "DELETE FROM concept_communities
         WHERE concept_name = ?1
           AND NOT EXISTS (
               SELECT 1 FROM concept_mentions WHERE concept_name = ?1
           )",
    )?;

    for concept in affected_concepts {
        delete_community_statement.execute(params![concept])?;
        delete_concept_statement.execute(params![concept])?;
    }

    Ok(())
}

pub(crate) fn purge_unreferenced_concepts(connection: &Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM concept_communities
         WHERE NOT EXISTS (
             SELECT 1 FROM concept_mentions
             WHERE concept_mentions.concept_name = concept_communities.concept_name
         )",
        [],
    )?;
    connection.execute(
        "DELETE FROM concepts
         WHERE NOT EXISTS (
             SELECT 1 FROM concept_mentions
             WHERE concept_mentions.concept_name = concepts.concept_name
         )",
        [],
    )?;
    Ok(())
}

// T-13.6: RTM NFR-1 — loads source files containing any of the affected concepts
pub(crate) fn load_candidate_files_for_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let mut candidate_files = BTreeSet::new();
    let mut statement = connection
        .prepare("SELECT DISTINCT source_file FROM concept_mentions WHERE concept_name = ?1")?;

    for concept in affected_concepts {
        let rows = statement.query_map(params![concept], |row| row.get::<_, String>(0))?;
        for file_uri in rows {
            candidate_files.insert(file_uri?);
        }
    }

    Ok(candidate_files)
}

fn write_relation_counts(connection: &Connection, relation_counts: RelationCounts) -> Result<()> {
    for ((concept_a, concept_b), (count, files)) in relation_counts {
        let source_files = serde_json::to_string(&files)?;
        connection.execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
            params![concept_a, concept_b, count, source_files],
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "graph_state_tests.rs"]
mod tests;
