use chrono::{DateTime, Duration, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::from_str;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_SUBFOLDER: &str = "tests/incremental-unify";

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

fn sync_path(cfg: &Config, path: &Path) -> String {
    let context = ToolContext::new(cfg.clone(), maenifold_core::json::OutputFormat::Markdown);
    sync_single_file(&context, path).expect("sync single file")
}

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn create_test_file(cfg: &Config, content: &str, name_prefix: Option<&str>) -> (PathBuf, String) {
    let test_folder = cfg.memory_path.join(TEST_SUBFOLDER);
    fs::create_dir_all(&test_folder).expect("mkdir");

    let unique_suffix = uuid::Uuid::new_v4().simple().to_string();
    let prefix = name_prefix.unwrap_or("note");
    let file_path = test_folder.join(format!("{prefix}-{unique_suffix}.md"));
    fs::write(&file_path, content).expect("write fixture");

    let memory_uri = path_to_uri(&file_path, &cfg.memory_path).expect("file uri");
    (file_path, memory_uri)
}

#[derive(Debug)]
struct FileContentRow {
    last_indexed: Option<String>,
    file_md5: Option<String>,
    file_size: Option<i64>,
}

fn get_file_content_row(cfg: &Config, memory_uri: &str) -> Option<FileContentRow> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = ?1",
            [memory_uri],
            |row| {
                Ok(FileContentRow {
                    last_indexed: row.get(0)?,
                    file_md5: row.get(1)?,
                    file_size: row.get(2)?,
                })
            },
        )
        .optional()
        .expect("query file_content")
}

fn get_mention_count(cfg: &Config, memory_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query concept_mentions")
}

fn get_edge_source_files(
    cfg: &Config,
    concept_a: &str,
    concept_b: &str,
) -> Option<BTreeMap<String, f64>> {
    let (left, right) = if concept_a <= concept_b {
        (concept_a, concept_b)
    } else {
        (concept_b, concept_a)
    };

    db_connection(cfg)
        .query_row(
            "SELECT source_files FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("query concept_graph")
        .and_then(|raw| {
            if raw.trim().is_empty() {
                None
            } else {
                Some(from_str::<BTreeMap<String, f64>>(&raw).expect("parse source_files"))
            }
        })
}

fn tamper_delete_mentions_and_edges_for_file(cfg: &Config, memory_uri: &str) {
    let connection = db_connection(cfg);
    connection
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![memory_uri],
        )
        .expect("delete concept_mentions");

    let mut statement = connection
        .prepare("SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE ?1")
        .expect("prepare edge query");
    let edges = statement
        .query_map([format!("%{memory_uri}%")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .expect("query edges")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect edges");

    for (concept_a, concept_b, files) in edges {
        let mut file_dict =
            from_str::<BTreeMap<String, f64>>(&files).unwrap_or_else(|_| BTreeMap::new());
        file_dict.remove(memory_uri);

        if file_dict.is_empty() {
            connection
                .execute(
                    "DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
                    params![concept_a, concept_b],
                )
                .expect("delete empty edge");
            continue;
        }

        connection
            .execute(
                "UPDATE concept_graph SET co_occurrence_count = ?3, source_files = ?4 WHERE concept_a = ?1 AND concept_b = ?2",
                params![
                    concept_a,
                    concept_b,
                    file_dict.len() as i64,
                    serde_json::to_string(&file_dict).expect("serialize source_files"),
                ],
            )
            .expect("update edge");
    }
}

fn parse_rfc3339_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("parse timestamp")
        .with_timezone(&Utc)
}

fn set_file_timestamp(path: &Path, timestamp: DateTime<Utc>) {
    let file_time =
        FileTime::from_unix_time(timestamp.timestamp(), timestamp.timestamp_subsec_nanos());
    set_file_mtime(path, file_time).expect("set file mtime");
}

// T-5.7: RTM FR-14.3
#[test]
fn incremental_sync_file_changed_creates_expected_db_artifacts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-Alpha-{unique}");
    let concept_b = format!("Incremental-Unify-Beta-{unique}");
    let content =
        format!("# Unification Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");

    let (file_path, memory_uri) = create_test_file(&cfg, &content, Some(&format!("incr-{unique}")));
    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("concept mentions updated"),
        "incremental sync should process the file: {output}"
    );

    let file_content = get_file_content_row(&cfg, &memory_uri);
    let mention_a = get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a));
    let mention_b = get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b));

    assert!(
        file_content.is_some(),
        "incremental sync should create file_content row"
    );
    assert_eq!(
        mention_a,
        Some(1),
        "mention count for concept A should be 1"
    );
    assert_eq!(
        mention_b,
        Some(1),
        "mention count for concept B should be 1"
    );

    let file_content = file_content.expect("file_content");
    assert!(
        file_content
            .file_md5
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        "incremental sync must populate file_md5"
    );
    assert!(
        file_content.file_size.is_some_and(|value| value > 0),
        "incremental sync must populate file_size"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn incremental_sync_file_deleted_cleans_all_artifacts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-DelA-{unique}");
    let concept_b = format!("Incremental-Unify-DelB-{unique}");
    let content = format!("# Delete Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    assert!(
        get_file_content_row(&cfg, &memory_uri).is_some(),
        "file should be indexed before deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_some(),
        "mention A should exist before deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)).is_some(),
        "mention B should exist before deletion"
    );

    fs::remove_file(&file_path).expect("delete file");
    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed state: {output}"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "concept_mentions for concept A must be removed after deletion"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)).is_none(),
        "concept_mentions for concept B must be removed after deletion"
    );
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_none(),
        "file_content row must be removed after deletion"
    );

    let edge = get_edge_source_files(
        &cfg,
        &normalize_concept(&concept_a),
        &normalize_concept(&concept_b),
    );
    assert!(
        edge.as_ref()
            .is_none_or(|files| !files.contains_key(&memory_uri)),
        "concept_graph edge must not reference the deleted file"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn incremental_sync_file_renamed_reindexes_under_new_uri() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-RenA-{unique}");
    let concept_b = format!("Incremental-Unify-RenB-{unique}");
    let content = format!("# Rename Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_some(),
        "original file should be indexed"
    );

    let test_folder = file_path.parent().expect("parent").to_path_buf();
    let new_file_path = test_folder.join(format!("renamed-{unique}.md"));
    fs::rename(&file_path, &new_file_path).expect("rename file");
    let new_memory_uri = path_to_uri(&new_file_path, &cfg.memory_path).expect("new uri");

    let delete_output = sync_path(&cfg, &file_path);
    let create_output = sync_path(&cfg, &new_file_path);

    assert!(
        delete_output.contains("removed stale index entry"),
        "old path should be deleted after rename: {delete_output}"
    );
    assert!(
        create_output.contains("concept mentions updated"),
        "new path should be indexed after rename: {create_output}"
    );
    assert!(
        get_file_content_row(&cfg, &memory_uri).is_none(),
        "old URI must be removed from file_content after rename"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "old URI concept_mentions must be removed after rename"
    );
    assert!(
        get_file_content_row(&cfg, &new_memory_uri).is_some(),
        "new URI must be present in file_content after rename"
    );
    assert!(
        get_mention_count(&cfg, &new_memory_uri, &normalize_concept(&concept_a)).is_some(),
        "new URI must have concept_mentions for concept A after rename"
    );
    assert!(
        get_mention_count(&cfg, &new_memory_uri, &normalize_concept(&concept_b)).is_some(),
        "new URI must have concept_mentions for concept B after rename"
    );
}

// T-5.7: RTM FR-14.1
#[test]
fn incremental_sync_mtime_guard_skips_unchanged_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-MtA-{unique}");
    let concept_common = format!("Incremental-Unify-MtCommon-{unique}");
    let content =
        format!("# Mtime Guard Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    let row_after_create = get_file_content_row(&cfg, &memory_uri).expect("indexed row");
    let stored_mtime = parse_rfc3339_utc(
        row_after_create
            .last_indexed
            .as_deref()
            .expect("stored last_indexed"),
    );
    set_file_timestamp(&file_path, stored_mtime);

    tamper_delete_mentions_and_edges_for_file(&cfg, &memory_uri);
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "mentions should be tampered away before re-sync"
    );

    let output = sync_path(&cfg, &file_path);

    assert!(
        output.contains("mtime guard"),
        "same file mtime should skip reprocessing: {output}"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "when mtime is unchanged, sync_single_file must skip and must not restore missing mentions"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalize_concept(&concept_a),
            &normalize_concept(&concept_common),
        )
        .is_none(),
        "when mtime is unchanged, sync_single_file must not restore missing graph edges"
    );
}

// T-5.7: RTM FR-14.2
#[test]
fn incremental_sync_hash_guard_updates_last_indexed_only() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let concept_a = format!("Incremental-Unify-HgA-{unique}");
    let concept_common = format!("Incremental-Unify-HgCommon-{unique}");
    let content =
        format!("# Hash Guard Test\n\nReferences [[{concept_a}]] and [[{concept_common}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, &content, None);

    sync_path(&cfg, &file_path);

    let before = get_file_content_row(&cfg, &memory_uri).expect("indexed row");
    let original_hash = before.file_md5.clone().expect("stored hash");
    let previous_last_indexed =
        parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"));

    tamper_delete_mentions_and_edges_for_file(&cfg, &memory_uri);
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "mentions should be tampered away before re-sync"
    );

    let new_mtime = previous_last_indexed + Duration::minutes(1);
    set_file_timestamp(&file_path, new_mtime);

    let output = sync_path(&cfg, &file_path);
    let after = get_file_content_row(&cfg, &memory_uri).expect("updated row");

    assert!(
        output.contains("hash guard"),
        "same content should skip on hash guard: {output}"
    );
    assert_ne!(
        after.last_indexed, before.last_indexed,
        "when mtime differs but hash is unchanged, sync_single_file must update last_indexed"
    );
    assert_eq!(
        after.file_md5.as_deref(),
        Some(original_hash.as_str()),
        "when content hash is unchanged, sync_single_file must not update file_md5"
    );
    assert!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)).is_none(),
        "when mtime differs but hash is unchanged, sync_single_file must not restore missing mentions"
    );
    assert!(
        get_edge_source_files(
            &cfg,
            &normalize_concept(&concept_a),
            &normalize_concept(&concept_common),
        )
        .is_none(),
        "when mtime differs but hash is unchanged, sync_single_file must not restore missing graph edges"
    );
}
