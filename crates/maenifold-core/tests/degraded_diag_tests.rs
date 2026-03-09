#![cfg(unix)]

use maenifold_core::assets::{AssetKind, AssetManager};
use maenifold_core::json::{OutputFormat, get_string_array};
use maenifold_core::memory::list_memories;
use maenifold_core::search::build_context;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, MaenifoldError};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn setup() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self { _tmp: tmp, cfg }
    }

    fn markdown_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn json_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn capture_stderr<F, T>(operation: F) -> (T, String)
where
    F: FnOnce() -> T,
{
    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(src: i32, dest: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let stderr = std::io::stderr();
    let stderr_fd = stderr.as_raw_fd();
    let mut pipe_fds = [0_i32; 2];

    // SAFETY: pipe_fds points to writable memory for two descriptors.
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");
    // SAFETY: stderr_fd is a valid descriptor.
    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");
    // SAFETY: dup2 atomically redirects the current process stderr descriptor.
    assert!(unsafe { dup2(pipe_fds[1], stderr_fd) } >= 0, "dup2 failed");

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    // SAFETY: restore the saved stderr and close owned write descriptors.
    assert!(
        unsafe { dup2(saved_stderr, stderr_fd) } >= 0,
        "restore dup2 failed"
    );
    unsafe {
        close(pipe_fds[1]);
        close(saved_stderr);
    }

    let mut output = String::new();
    // SAFETY: pipe_fds[0] is the remaining owned read descriptor.
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}

fn write_memory_file(env: &TestEnv, relative_path: &str, content: &str) -> PathBuf {
    let path = env.cfg.memory_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&path, content).expect("write memory file");
    path
}

fn open_connection(env: &TestEnv) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(&env.cfg.database_path).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .expect("pragmas");
    conn
}

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
    assert!(
        stderr.contains("[DEGRADED][AssetResources][GetAssetMetadata]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

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
        !direct.is_empty(),
        "direct relations should still be populated: {output}"
    );
    assert!(
        stderr.contains("[DEGRADED][GraphTools][BuildContext]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_EnrichWithCommunityData_EmitsDegradedOnMissingTable.
#[test]
fn enrich_with_community_data_emits_degraded_on_missing_table() {
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
    assert!(
        stderr.contains("[DEGRADED][GraphTools][EnrichWithCommunityData]"),
        "expected degraded diagnostic, got: {stderr}"
    );
}

// T-0.1: RTM FR-25.2 ports FR25_2_AC3_PayloadReader_EmitsDegradedOnMalformedJsonString.
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
