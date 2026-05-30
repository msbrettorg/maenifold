use super::*;
use crate::{
    config::Config,
    db::{self, GraphDatabase},
    test_util::ConfigResetGuard,
};
use rusqlite::{Connection, OptionalExtension, params};
use serial_test::serial;
use std::collections::{BTreeMap, BTreeSet};
use tempfile::TempDir;

fn setup_graph_db() -> (TempDir, ConfigResetGuard, Connection) {
    ConfigResetGuard::acquire_test_lock();
    let guard = ConfigResetGuard;
    let temp_dir = TempDir::new().expect("create temp dir for graph-state tests");
    Config::override_root(temp_dir.path());
    Config::set_database_path(temp_dir.path().join("graph-state-tests.db"));
    let config = Config::current();
    GraphDatabase::initialize_database(&config).expect("initialize real maenifold schema");
    let connection =
        db::open_connection(&config.database_path).expect("open initialized graph-state database");
    (temp_dir, guard, connection)
}

fn insert_file_content(connection: &Connection, file_path: &str, content: &str) {
    connection
        .execute(
            "INSERT INTO file_content (file_path, title, content) VALUES (?1, ?2, ?3)",
            params![file_path, file_path, content],
        )
        .expect("insert file_content row");
}

fn insert_mention(connection: &Connection, concept: &str, source_file: &str, mention_count: i64) {
    connection
        .execute(
            "INSERT INTO concept_mentions (concept_name, source_file, mention_count) VALUES (?1, ?2, ?3)",
            params![concept, source_file, mention_count],
        )
        .expect("insert concept mention");
}

fn insert_concept(
    connection: &Connection,
    concept: &str,
    first_seen: Option<&str>,
    occurrence_count: i64,
) {
    connection
        .execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)",
            params![concept, first_seen, occurrence_count],
        )
        .expect("insert concept row");
}

fn insert_community(connection: &Connection, concept: &str) {
    connection
        .execute(
            "INSERT INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![concept, 1_i64, 0.75_f64, 1.0_f64, "2025-01-01T00:00:00Z"],
        )
        .expect("insert concept community");
}

fn update_file_content(connection: &Connection, file_path: &str, content: &str) {
    connection
        .execute(
            "UPDATE file_content SET content = ?2 WHERE file_path = ?1",
            params![file_path, content],
        )
        .expect("update file_content row");
}

fn delete_mention(connection: &Connection, concept: &str, source_file: &str) {
    connection
        .execute(
            "DELETE FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept, source_file],
        )
        .expect("delete concept mention");
}

fn concept_row(connection: &Connection, concept: &str) -> Option<(Option<String>, i64)> {
    connection
        .query_row(
            "SELECT first_seen, occurrence_count FROM concepts WHERE concept_name = ?1",
            params![concept],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .expect("query concept row")
}

fn community_exists(connection: &Connection, concept: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM concept_communities WHERE concept_name = ?1",
            params![concept],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query concept community")
        .is_some()
}

fn graph_rows(connection: &Connection) -> Vec<(String, String, i64, BTreeMap<String, f64>)> {
    let mut statement = connection
        .prepare(
            "SELECT concept_a, concept_b, co_occurrence_count, source_files
             FROM concept_graph
             ORDER BY concept_a, concept_b",
        )
        .expect("prepare concept_graph query");
    statement
        .query_map([], |row| {
            let source_files: String = row.get(3)?;
            let parsed = serde_json::from_str::<BTreeMap<String, f64>>(&source_files)
                .expect("parse concept_graph.source_files");
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, parsed))
        })
        .expect("query concept_graph rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect concept_graph rows")
}

#[test]
#[serial]
fn rebuild_graph_from_mentions_preserves_first_seen_and_purges_unreferenced_rows() {
    let (_temp_dir, _guard, connection) = setup_graph_db();
    let indexed_at = "2025-01-02T00:00:00Z";
    let alpha_first_seen = "2024-12-31T23:59:59Z";

    insert_file_content(&connection, "memory://file-1.md", "[[alpha]] [[beta]]");
    insert_file_content(&connection, "memory://file-2.md", "[[alpha]] [[gamma]]");
    insert_concept(&connection, "alpha", Some(alpha_first_seen), 99);
    insert_concept(&connection, "beta", None, 0);
    insert_concept(&connection, "gamma", None, 0);
    insert_concept(&connection, "orphan", Some("2024-12-01T00:00:00Z"), 7);
    insert_mention(&connection, "alpha", "memory://file-1.md", 2);
    insert_mention(&connection, "beta", "memory://file-1.md", 1);
    insert_mention(&connection, "alpha", "memory://file-2.md", 1);
    insert_mention(&connection, "gamma", "memory://file-2.md", 4);

    insert_community(&connection, "orphan");
    connection
        .execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
             VALUES (?1, ?2, ?3, ?4)",
            params!["alpha", "orphan", 9, r#"{"memory://stale.md":1.0}"#],
        )
        .expect("insert stale concept_graph row");

    let total_mentions =
        rebuild_graph_from_mentions(&connection, indexed_at).expect("rebuild graph from mentions");

    assert_eq!(total_mentions, 8);
    assert_eq!(
        concept_row(&connection, "alpha"),
        Some((Some(alpha_first_seen.to_string()), 3))
    );
    assert_eq!(
        concept_row(&connection, "beta"),
        Some((Some(indexed_at.to_string()), 1))
    );
    assert_eq!(
        concept_row(&connection, "gamma"),
        Some((Some(indexed_at.to_string()), 4))
    );
    assert_eq!(concept_row(&connection, "orphan"), None);
    assert!(!community_exists(&connection, "orphan"));
    assert_eq!(
        graph_rows(&connection),
        vec![
            (
                "alpha".to_string(),
                "beta".to_string(),
                1,
                BTreeMap::from([("memory://file-1.md".to_string(), 0.5)]),
            ),
            (
                "alpha".to_string(),
                "gamma".to_string(),
                1,
                BTreeMap::from([("memory://file-2.md".to_string(), 0.5)]),
            ),
        ]
    );
}

#[test]
#[serial]
fn load_existing_state_returns_seeded_rows_and_deduplicates_relations() {
    let (_temp_dir, _guard, connection) = setup_graph_db();

    insert_concept(&connection, "alpha", Some("2024-01-01T00:00:00Z"), 1);
    insert_concept(&connection, "beta", Some("2024-01-02T00:00:00Z"), 2);
    insert_concept(&connection, "gamma", None, 3);
    connection
        .execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
             VALUES (?1, ?2, NULL, NULL)",
            params!["alpha", "beta"],
        )
        .expect("insert nullable concept_graph row");
    connection
        .execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
             VALUES (?1, ?2, ?3, ?4)",
            params!["beta", "gamma", 2, r#"{"memory://seed.md":1.0}"#],
        )
        .expect("insert concept_graph row");

    let first_seen = load_existing_first_seen(&connection).expect("load existing first_seen map");
    let relations = load_existing_relations_for_concepts(
        &connection,
        &BTreeSet::from(["beta".to_string(), "gamma".to_string()]),
    )
    .expect("load existing relations");

    assert_eq!(
        first_seen,
        BTreeMap::from([
            ("alpha".to_string(), "2024-01-01T00:00:00Z".to_string()),
            ("beta".to_string(), "2024-01-02T00:00:00Z".to_string()),
        ])
    );
    assert_eq!(relations.len(), 2);
    let alpha_beta = relations
        .get(&("alpha".to_string(), "beta".to_string()))
        .expect("alpha-beta relation should be loaded");
    assert_eq!(alpha_beta.co_occurrence_count, 0.0);
    assert_eq!(alpha_beta.source_files, "");

    let beta_gamma = relations
        .get(&("beta".to_string(), "gamma".to_string()))
        .expect("beta-gamma relation should be loaded");
    assert_eq!(beta_gamma.co_occurrence_count, 2.0);
    assert_eq!(beta_gamma.source_files, r#"{"memory://seed.md":1.0}"#);
}

#[test]
#[serial]
fn refresh_affected_concepts_updates_counts_inserts_new_rows_and_deletes_missing_ones() {
    let (_temp_dir, _guard, connection) = setup_graph_db();
    let indexed_at = "2025-01-03T00:00:00Z";
    let alpha_first_seen = "2024-06-01T12:00:00Z";
    let delta_first_seen = "2024-06-02T12:00:00Z";

    insert_file_content(&connection, "memory://file-1.md", "[[alpha]]");
    insert_file_content(&connection, "memory://file-2.md", "[[epsilon]]");
    insert_concept(&connection, "alpha", Some(alpha_first_seen), 1);
    insert_concept(&connection, "delta", Some(delta_first_seen), 3);
    insert_concept(&connection, "epsilon", None, 0);
    insert_community(&connection, "delta");
    insert_mention(&connection, "alpha", "memory://file-1.md", 5);
    insert_mention(&connection, "epsilon", "memory://file-2.md", 2);

    let existing_first_seen =
        load_existing_first_seen(&connection).expect("load first_seen before refresh");
    refresh_affected_concepts(
        &connection,
        &BTreeSet::from([
            "alpha".to_string(),
            "delta".to_string(),
            "epsilon".to_string(),
        ]),
        &existing_first_seen,
        indexed_at,
    )
    .expect("refresh affected concepts");

    assert_eq!(
        concept_row(&connection, "alpha"),
        Some((Some(alpha_first_seen.to_string()), 5))
    );
    assert_eq!(
        concept_row(&connection, "epsilon"),
        Some((Some(indexed_at.to_string()), 2))
    );
    assert_eq!(concept_row(&connection, "delta"), None);
    assert!(!community_exists(&connection, "delta"));
}

#[test]
#[serial]
fn prune_orphaned_concepts_only_removes_requested_orphans() {
    let (_temp_dir, _guard, connection) = setup_graph_db();

    insert_file_content(&connection, "memory://live.md", "[[live]]");
    insert_concept(&connection, "live", Some("2024-01-01T00:00:00Z"), 1);
    insert_concept(&connection, "orphan", Some("2024-01-01T00:00:00Z"), 1);
    insert_concept(&connection, "untouched", Some("2024-01-01T00:00:00Z"), 1);
    insert_community(&connection, "live");
    insert_community(&connection, "orphan");
    insert_community(&connection, "untouched");
    insert_mention(&connection, "live", "memory://live.md", 1);

    prune_orphaned_concepts(
        &connection,
        &BTreeSet::from(["live".to_string(), "orphan".to_string()]),
    )
    .expect("prune orphaned concepts");

    assert!(concept_row(&connection, "live").is_some());
    assert!(community_exists(&connection, "live"));
    assert_eq!(concept_row(&connection, "orphan"), None);
    assert!(!community_exists(&connection, "orphan"));
    assert!(concept_row(&connection, "untouched").is_some());
    assert!(community_exists(&connection, "untouched"));
}

#[test]
#[serial]
fn rebuild_affected_graph_from_mentions_updates_only_touched_concepts_and_relations() {
    let (_temp_dir, _guard, connection) = setup_graph_db();
    let initial_indexed_at = "2025-01-01T00:00:00Z";
    let refresh_indexed_at = "2025-01-04T00:00:00Z";

    insert_file_content(&connection, "memory://file-1.md", "[[alpha]] [[beta]]");
    insert_file_content(&connection, "memory://file-2.md", "[[alpha]] [[gamma]]");
    insert_file_content(&connection, "memory://file-9.md", "[[omega]] [[psi]]");
    insert_concept(&connection, "alpha", None, 0);
    insert_concept(&connection, "beta", None, 0);
    insert_concept(&connection, "gamma", None, 0);
    insert_concept(&connection, "omega", None, 0);
    insert_concept(&connection, "psi", None, 0);
    insert_mention(&connection, "alpha", "memory://file-1.md", 1);
    insert_mention(&connection, "beta", "memory://file-1.md", 1);
    insert_mention(&connection, "alpha", "memory://file-2.md", 1);
    insert_mention(&connection, "gamma", "memory://file-2.md", 1);
    insert_mention(&connection, "omega", "memory://file-9.md", 1);
    insert_mention(&connection, "psi", "memory://file-9.md", 1);
    rebuild_graph_from_mentions(&connection, initial_indexed_at)
        .expect("build initial graph state from mentions");

    delete_mention(&connection, "beta", "memory://file-1.md");
    update_file_content(&connection, "memory://file-1.md", "[[alpha]]");
    insert_file_content(&connection, "memory://file-3.md", "[[alpha]] [[delta]]");
    insert_concept(&connection, "delta", None, 0);
    insert_mention(&connection, "alpha", "memory://file-3.md", 1);
    insert_mention(&connection, "delta", "memory://file-3.md", 1);

    let affected_concepts =
        BTreeSet::from(["alpha".to_string(), "beta".to_string(), "delta".to_string()]);
    let candidate_files = load_candidate_files_for_concepts(&connection, &affected_concepts)
        .expect("load candidate files");
    let updated_relations =
        rebuild_affected_graph_from_mentions(&connection, &affected_concepts, refresh_indexed_at)
            .expect("rebuild affected graph state from mentions");

    assert_eq!(
        candidate_files,
        BTreeSet::from([
            "memory://file-1.md".to_string(),
            "memory://file-2.md".to_string(),
            "memory://file-3.md".to_string(),
        ])
    );
    assert_eq!(updated_relations, 2);
    assert_eq!(concept_row(&connection, "beta"), None);
    assert_eq!(
        concept_row(&connection, "delta"),
        Some((Some(refresh_indexed_at.to_string()), 1))
    );
    assert_eq!(
        concept_row(&connection, "alpha"),
        Some((Some(initial_indexed_at.to_string()), 3))
    );
    assert_eq!(
        graph_rows(&connection),
        vec![
            (
                "alpha".to_string(),
                "delta".to_string(),
                1,
                BTreeMap::from([("memory://file-3.md".to_string(), 0.5)]),
            ),
            (
                "alpha".to_string(),
                "gamma".to_string(),
                1,
                BTreeMap::from([("memory://file-2.md".to_string(), 0.5)]),
            ),
            (
                "omega".to_string(),
                "psi".to_string(),
                1,
                BTreeMap::from([("memory://file-9.md".to_string(), 0.5)]),
            ),
        ]
    );
}
