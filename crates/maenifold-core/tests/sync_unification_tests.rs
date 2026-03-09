use chrono::{DateTime, Duration, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search::{sync, sync_single_file};
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn sync_one(cfg: &Config, path: &Path) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, path).expect("sync single file")
}

fn sync_all(cfg: &Config) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync(&context, &json!({})).expect("sync all")
}

fn create_test_markdown_file(cfg: &Config, file_name: &str, content: &str) -> (PathBuf, String) {
    let path = cfg
        .memory_path
        .join("tests")
        .join("sync-unification")
        .join(file_name);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

#[derive(Debug, Clone)]
struct FileContentRow {
    last_indexed: Option<String>,
    file_md5: Option<String>,
    file_size: Option<i64>,
}

fn get_file_content_row(cfg: &Config, file_uri: &str) -> Option<FileContentRow> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = ?1",
            [file_uri],
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

fn get_mention_count(cfg: &Config, file_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, file_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query mention_count")
}

fn tamper_delete_mentions(cfg: &Config, file_uri: &str) {
    db_connection(cfg)
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![file_uri],
        )
        .expect("delete mentions");
}

fn parse_rfc3339_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("parse rfc3339")
        .with_timezone(&Utc)
}

fn set_file_mtime_utc(path: &Path, instant: DateTime<Utc>) {
    set_file_mtime(
        path,
        FileTime::from_unix_time(instant.timestamp(), instant.timestamp_subsec_nanos()),
    )
    .expect("set mtime");
}

fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}

// T-5.7: RTM FR-14.3
#[test]
fn process_file_new_file_inserts_content_mentions_and_index_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "new-file.md",
        &format!("# Unification Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("Synced memory://tests/sync-unification/new-file"),
        "new file should be processed: {output}"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "file_content row must exist after sync_single_file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "concept_mentions must contain concept_a with count 1"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        Some(1),
        "concept_mentions must contain concept_b with count 1"
    );
}

// T-5.7: RTM FR-14.1
#[test]
fn process_file_mtime_unchanged_skips_without_restoring_deleted_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "mtime-unchanged.md",
        &format!("# Skip Test\n\nReferences [[{concept_a}]].\n"),
    );

    let initial = sync_one(&cfg, &path);
    assert!(
        initial.contains("Synced memory://tests/sync-unification/mtime-unchanged"),
        "initial sync should index the file: {initial}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "precondition: mention must exist after initial sync"
    );

    tamper_delete_mentions(&cfg, &file_uri);
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "precondition: mentions must be absent after tamper"
    );

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("mtime guard"),
        "mtime guard should skip unchanged file: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "mentions must remain deleted when mtime is unchanged"
    );
}

// T-5.7: RTM FR-14.2
#[test]
fn process_file_mtime_differs_but_hash_same_updates_last_indexed_only() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "hash-guard.md",
        &format!("# Hash Guard Test\n\nReferences [[{concept_a}]].\n"),
    );

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");

    tamper_delete_mentions(&cfg, &file_uri);
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + Duration::minutes(2);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("hash guard"),
        "mtime-only changes should skip reprocessing on identical content: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_eq!(
        after.last_indexed,
        Some(file_mtime_rfc3339(&path)),
        "last_indexed must update to the new filesystem mtime"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "file_md5 must remain unchanged when content is identical"
    );
    assert_eq!(
        after.file_size, before.file_size,
        "file_size must remain unchanged when content is identical"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "mentions must remain deleted when the hash guard skips reprocessing"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn process_file_content_changed_reprocesses_all_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let concept_c = "sync-unify-c";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "content-change.md",
        &format!("# Content Change Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    sync_one(&cfg, &path);
    let before = get_file_content_row(&cfg, &file_uri).expect("row before");
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1)
    );

    fs::write(
        &path,
        format!(
            "# Content Change Test\n\nReferences [[{concept_a}]], [[{concept_b}]], and [[{concept_c}]].\n"
        ),
    )
    .expect("rewrite content");
    let new_mtime = parse_rfc3339_utc(before.last_indexed.as_deref().expect("stored last_indexed"))
        + Duration::minutes(1);
    set_file_mtime_utc(&path, new_mtime);

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("concept mentions updated"),
        "content changes should trigger reprocessing: {output}"
    );

    let after = get_file_content_row(&cfg, &file_uri).expect("row after");
    assert_ne!(
        after.file_md5, before.file_md5,
        "file_md5 must change when file content changes"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1)
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        Some(1)
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_c)),
        Some(1),
        "new concept must appear in mentions after reprocessing"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn remove_file_cleans_content_and_mentions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "remove-file.md",
        &format!("# Removal Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"),
    );

    sync_one(&cfg, &path);
    assert!(
        get_file_content_row(&cfg, &file_uri).is_some(),
        "precondition: file_content must exist"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        Some(1),
        "precondition: mentions must exist"
    );

    fs::remove_file(&path).expect("delete target file");
    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed concepts: {output}"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri).is_none(),
        "file_content row must be deleted after syncing a removed file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_a)),
        None,
        "concept_mentions must be deleted after removing the file"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &normalize_concept(concept_b)),
        None,
        "concept_mentions must be deleted after removing the file"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn sync_batch_processes_multiple_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = "sync-unify-a";
    let concept_b = "sync-unify-b";
    let concept_c = "sync-unify-c";
    let (_path1, file_uri1) = create_test_markdown_file(
        &cfg,
        "batch-1.md",
        &format!("# Batch 1\n\nReferences [[{concept_a}]].\n"),
    );
    let (_path2, file_uri2) = create_test_markdown_file(
        &cfg,
        "batch-2.md",
        &format!("# Batch 2\n\nReferences [[{concept_b}]].\n"),
    );
    let (_path3, file_uri3) = create_test_markdown_file(
        &cfg,
        "batch-3.md",
        &format!("# Batch 3\n\nReferences [[{concept_c}]].\n"),
    );

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "batch sync should complete successfully: {output}"
    );
    assert!(
        output.contains("- 3 FILES processed"),
        "batch sync should process all files in the fixture set: {output}"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri1).is_some(),
        "file 1 must have a file_content row"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri2).is_some(),
        "file 2 must have a file_content row"
    );
    assert!(
        get_file_content_row(&cfg, &file_uri3).is_some(),
        "file 3 must have a file_content row"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_a)),
        Some(1),
        "file 1 mentions must be recorded"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri2, &normalize_concept(concept_b)),
        Some(1),
        "file 2 mentions must be recorded"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri3, &normalize_concept(concept_c)),
        Some(1),
        "file 3 mentions must be recorded"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn sync_empty_memory_directory_returns_without_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "empty sync should still return a report: {output}"
    );
    assert!(
        output.contains("- 0 FILES processed"),
        "empty sync should report zero processed files: {output}"
    );
}
