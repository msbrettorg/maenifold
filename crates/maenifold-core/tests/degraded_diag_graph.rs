#![cfg(unix)]

// T-12.7: RTM NFR-1
#[path = "degraded_diag/support.rs"]
mod support;

use maenifold_core::json::get_string_array;
use maenifold_core::search::build_context;
use rusqlite::params;
use serde_json::{Value, json};
use support::{TEST_MUTEX, TestEnv, capture_stderr, open_connection};

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_BuildContext_EmitsDegradedOnUnreadableFile.
#[test]
fn build_context_emits_degraded_on_unreadable_file() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_connection(&env);
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params!["ctx-a"],
    )
    .expect("insert concept a");
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params!["ctx-b"],
    )
    .expect("insert concept b");
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
        params!["ctx-a", "ctx-b", 3_i64, r#"{"memory://ctx-a": 1.0}"#],
    )
    .expect("insert edge");
    conn.execute(
        "INSERT OR REPLACE INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
         VALUES (?1, '', ?2, '', datetime('now'), 'saved', '', 0, datetime('now'), NULL)",
        params!["memory://ctx-a", rusqlite::types::Value::Blob(vec![0xFF, 0xFE, 0xFD])],
    )
    .expect("insert unreadable file_content row");
    drop(conn);

    let (result, stderr) = capture_stderr(|| {
        build_context(
            &env.json_context(),
            &json!({ "conceptName": "ctx-a", "depth": 1, "includeContent": true }),
        )
    });

    let output = result.expect("BuildContext must degrade gracefully");
    let parsed: Value = serde_json::from_str(&output).expect("parse json");
    let direct = parsed["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations");
    assert!(
        direct.iter().any(|r| r["Name"].as_str() == Some("ctx-b")),
        "DirectRelations must contain ctx-b despite content-read failure: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][GraphTools][BuildContext]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_EnrichWithCommunityData_EmitsDegradedOnMissingTable.
//
// Scenario: concept_communities is recreated with only (concept_name, detected_at),
// deliberately omitting community_id, modularity, and resolution. When
// GraphDatabase::initialize_database detects the incompatible schema it returns an
// error containing "concept_communities" or "community_id", which build_context
// catches and emits as a [DEGRADED] diagnostic rather than aborting.
#[test]
fn enrich_with_community_data_emits_degraded_on_incompatible_schema() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_connection(&env);
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params!["community-a"],
    )
    .expect("insert concept a");
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params!["community-b"],
    )
    .expect("insert concept b");
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
        params!["community-a", "community-b", 5_i64, r#"{"memory://community-a": 1.0}"#],
    )
    .expect("insert edge");
    // Recreate concept_communities with an intentionally incomplete schema.
    // The production table requires community_id INTEGER, modularity REAL, and
    // resolution REAL — all omitted here to trigger schema-validation failure.
    conn.execute_batch(
        "DROP TABLE concept_communities;
         CREATE TABLE concept_communities (
             concept_name TEXT PRIMARY KEY,
             detected_at TEXT NOT NULL
         );",
    )
    .expect("replace communities schema");
    drop(conn);

    let (result, stderr) = capture_stderr(|| {
        build_context(
            &env.json_context(),
            &json!({ "conceptName": "community-a", "depth": 1 }),
        )
    });

    let output = result.expect("BuildContext must degrade gracefully");
    let parsed: Value = serde_json::from_str(&output).expect("parse json");
    assert_eq!(parsed["data"]["ConceptName"], "community-a");
    let direct = parsed["data"]["DirectRelations"]
        .as_array()
        .expect("DirectRelations must be present");
    assert!(
        !direct.is_empty(),
        "graph relations must survive community schema failure: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][GraphTools][EnrichWithCommunityData]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_PayloadReader_EmitsDegradedOnMalformedJsonString.
// Note: get_string_array silently returns None for non-string/non-array inputs (null,
// number, bool, object) without emitting a diagnostic — that is intentional C# oracle
// parity. Only the Value::String arm emits [DEGRADED] when the string cannot be
// deserialized as Vec<String>.
#[test]
fn payload_reader_emits_degraded_on_malformed_json_string() {
    let _guard = TEST_MUTEX.lock();
    let payload = json!({ "items": "this-is-not-a-json-array" });

    let (result, stderr) = capture_stderr(|| get_string_array(&payload, "items"));

    assert!(
        result.is_none(),
        "expected malformed string array to degrade to None"
    );
    assert!(
        stderr.contains("[DEGRADED][PayloadReader][GetStringArray]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}
