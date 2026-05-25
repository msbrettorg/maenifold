use super::resources_tests_common::*;
use crate::db::{self, GraphDatabase};
use rmcp::model::{ReadResourceResult, ResourceContents};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub(super) const PRIMER_URI: &str = "graph://corpus/primer";
pub(super) const PRIMER_LIMIT_ONE_URI: &str = "graph://corpus/primer?limit=1";

pub(super) fn expected_lines() -> Vec<&'static str> {
    vec![
        "**[[gate]]** [[task-pm]] [[red-team]] [[blue-team]] [[confession-report]]",
        "**[[mcp]]** [[gateway]] [[resources]] [[tools]] [[sampling]]",
    ]
}

pub(super) fn setup_primer_fixture() -> (TempDir, ConfigResetGuard) {
    let (tmp, guard) = setup_test_env();
    GraphDatabase::initialize_database(&Config::current()).expect("initialize graph database");
    let connection =
        db::open_connection(&Config::current().database_path).expect("open graph database");
    seed_named_fixture(&connection);
    (tmp, guard)
}

pub(super) fn setup_large_primer_fixture() -> (TempDir, ConfigResetGuard) {
    let (tmp, guard) = setup_test_env();
    GraphDatabase::initialize_database(&Config::current()).expect("initialize graph database");
    let connection =
        db::open_connection(&Config::current().database_path).expect("open graph database");
    seed_large_fixture(&connection, 32, 12);
    (tmp, guard)
}

pub(super) fn setup_empty_primer_fixture() -> (TempDir, ConfigResetGuard) {
    let (tmp, guard) = setup_test_env();
    GraphDatabase::initialize_database(&Config::current()).expect("initialize graph database");
    (tmp, guard)
}

pub(super) fn extract_text(result: ReadResourceResult) -> String {
    match result.contents.first() {
        Some(ResourceContents::TextResourceContents { text, .. }) => text.clone(),
        other => panic!("expected text resource contents, got {other:?}"),
    }
}

pub(super) fn assert_payload_lines(text: &str, expected: &[&str]) {
    let payload: Value = serde_json::from_str(text).expect("primer payload JSON");
    assert_eq!(payload, json!({ "lines": expected }));
}

fn seed_named_fixture(connection: &Connection) {
    seed_community(
        connection,
        1,
        1,
        "gate",
        &[
            ("task-pm", 20),
            ("red-team", 18),
            ("blue-team", 16),
            ("confession-report", 14),
        ],
    );
    seed_community(
        connection,
        2,
        120,
        "mcp",
        &[
            ("gateway", 12),
            ("resources", 10),
            ("tools", 8),
            ("sampling", 6),
        ],
    );
}

fn seed_large_fixture(connection: &Connection, communities: usize, concepts_per_community: usize) {
    for community_id in 0..communities {
        let anchor = format!("community-{community_id:02}-anchor");
        let members = (0..concepts_per_community.saturating_sub(1))
            .map(|member_index| {
                (
                    format!("community-{community_id:02}-member-{member_index:02}"),
                    (concepts_per_community - member_index) as i64,
                )
            })
            .collect::<Vec<_>>();
        let borrowed = members
            .iter()
            .map(|(name, weight)| (name.as_str(), *weight))
            .collect::<Vec<_>>();
        seed_community(
            connection,
            community_id as i64 + 1,
            (community_id % 14) as i64,
            &anchor,
            &borrowed,
        );
    }
}

fn seed_community(
    connection: &Connection,
    community_id: i64,
    age_days: i64,
    anchor: &str,
    members: &[(&str, i64)],
) {
    insert_concept(connection, anchor, community_id, age_days);
    for (member, weight) in members {
        insert_concept(connection, member, community_id, age_days);
        connection
            .execute(
                "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                 VALUES (?1, ?2, ?3, ?4)",
                params![anchor, *member, *weight, format!("{anchor}.md,{member}.md")],
            )
            .expect("insert concept graph edge");
    }
}

fn insert_concept(connection: &Connection, concept: &str, community_id: i64, age_days: i64) {
    let timestamp = (chrono::Utc::now() - chrono::Duration::days(age_days)).to_rfc3339();
    let source_file = format!("memory://fixtures/{community_id}/{concept}.md");

    connection
        .execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count)
             VALUES (?1, ?2, 1)",
            params![concept, &timestamp],
        )
        .expect("insert concept");
    connection
        .execute(
            "INSERT INTO file_content (
                 file_path,
                 title,
                 content,
                 summary,
                 last_indexed,
                 status,
                 file_md5,
                 file_size,
                 created,
                 last_accessed
             ) VALUES (?1, ?2, '', '', ?3, 'ready', '', 0, ?3, ?3)",
            params![&source_file, concept, &timestamp],
        )
        .expect("insert file content");
    connection
        .execute(
            "INSERT INTO concept_mentions (concept_name, source_file, mention_count)
             VALUES (?1, ?2, 1)",
            params![concept, &source_file],
        )
        .expect("insert concept mention");
    connection
        .execute(
            "INSERT INTO concept_communities (
                 concept_name,
                 community_id,
                 modularity,
                 resolution,
                 detected_at
             ) VALUES (?1, ?2, 0.42, 1.0, ?3)",
            params![concept, community_id, &timestamp],
        )
        .expect("insert concept community");
}
