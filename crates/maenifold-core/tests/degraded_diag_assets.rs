#![cfg(unix)]

// T-12.7: RTM NFR-1
#[path = "degraded_diag/support.rs"]
mod support;

use maenifold_core::MaenifoldError;
use maenifold_core::assets::{AssetKind, AssetManager};
use maenifold_core::memory::list_memories;
use serde_json::{Value, json};
use std::fs;
use support::{TEST_MUTEX, TestEnv, capture_stderr, open_connection, write_memory_file};

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_LoadLastAccessedTimestamps_EmitsDegradedOnDbFailure.
#[test]
fn load_last_accessed_timestamps_emits_degraded_on_db_failure() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    fs::write(
        &env.cfg.database_path,
        "THIS IS NOT A VALID SQLITE DATABASE FILE",
    )
    .expect("corrupt db");

    let (result, stderr) = capture_stderr(|| list_memories(&env.markdown_context(), &json!({})));

    let output = result.expect("ListMemories must degrade gracefully");
    assert!(
        output.contains("# Directory: /"),
        "expected directory header despite degraded DB path: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][SystemTools][LoadLastAccessedTimestamps]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 — Covers Finding 1 (prepare-failure branch) in eval.
// Exercises the second emit site in load_last_accessed_timestamps: `connection.prepare(...)`
// fails because the `file_content` table has been dropped (schema-drift scenario).
#[test]
fn load_last_accessed_timestamps_emits_degraded_on_missing_table() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    // Drop the file_content table so that `connection.prepare(...)` fails with a "no such table" error.
    // The SQLite file remains valid — only the expected schema is absent.
    let conn = open_connection(&env);
    conn.execute_batch("DROP TABLE IF EXISTS file_content;")
        .expect("drop table");
    drop(conn);

    let (result, stderr) = capture_stderr(|| list_memories(&env.markdown_context(), &json!({})));

    let output = result.expect("ListMemories must degrade gracefully");
    assert!(
        output.contains("# Directory: /"),
        "expected directory header despite degraded DB path: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][SystemTools][LoadLastAccessedTimestamps]"),
        "expected degraded diagnostic for prepare failure, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_GetDecayMetadataForFile_EmitsDegradedOnFrontmatterFailure.
#[test]
fn get_decay_metadata_for_file_emits_degraded_on_frontmatter_failure() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    write_memory_file(
        &env,
        "broken-frontmatter.md",
        "---\ncreated: [not valid yaml\n---\n\n# Broken\n\ncontent",
    );

    let (result, stderr) = capture_stderr(|| list_memories(&env.markdown_context(), &json!({})));

    let output = result.expect("ListMemories must degrade gracefully");
    assert!(
        output.contains("📄 broken-frontmatter.md"),
        "expected malformed file to remain listed: {output}"
    );
    assert!(
        output.contains("decay_weight:"),
        "expected decay metadata output: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][SystemTools][GetDecayMetadataForFile]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_FindAssetFileById_EmitsDegradedOnMalformedJson.
#[test]
fn find_asset_file_by_id_emits_degraded_on_malformed_json() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let dir = env.cfg.assets_path.join("workflows");
    fs::create_dir_all(&dir).expect("asset dir");
    fs::write(
        dir.join("broken.json"),
        r#"{"id": BROKEN_VALUE_NOT_QUOTED}"#,
    )
    .expect("write broken");

    let manager = AssetManager::new(&env.cfg);
    let (result, stderr) = capture_stderr(|| manager.read_asset(AssetKind::Workflow, "missing-id"));

    match result {
        Err(MaenifoldError::AssetNotFound(kind, id)) => {
            assert_eq!(kind, "workflow");
            assert_eq!(id, "missing-id");
        }
        other => panic!("expected AssetNotFound, got {other:?}"),
    }
    assert!(
        stderr.contains("[DEGRADED][AssetResources][FindAssetFileById]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_GetAssetMetadata_EmitsDegradedOnMalformedJson.
#[test]
fn get_asset_metadata_emits_degraded_on_malformed_json() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let dir = env.cfg.assets_path.join("workflows");
    fs::create_dir_all(&dir).expect("asset dir");
    fs::write(
        dir.join("valid.json"),
        r#"{"id":"valid-1","name":"Valid Asset","description":"A valid asset"}"#,
    )
    .expect("write valid");
    fs::write(
        dir.join("broken.json"),
        r#"{ THIS IS NOT VALID JSON AT ALL }}}"#,
    )
    .expect("write broken");

    let manager = AssetManager::new(&env.cfg);
    let (result, stderr) = capture_stderr(|| manager.list_assets(Some(AssetKind::Workflow)));

    let value = result.expect("ListAssets must degrade gracefully");
    let items = value.as_array().expect("array");
    assert_eq!(
        items.len(),
        1,
        "expected only valid asset metadata: {value}"
    );
    assert_eq!(items[0]["id"], Value::String("valid-1".to_string()));
    // T-0.1: RTM FR-25.2 — Pin C# parity contract: multiAgent defaults to false when absent.
    assert_eq!(
        items[0]["multiAgent"],
        Value::Bool(false),
        "expected multiAgent to default to false for assets without the field"
    );
    assert!(
        stderr.contains("[DEGRADED][AssetResources][GetAssetMetadata]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}
