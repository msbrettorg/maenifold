use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn sync_all(env: &TestEnv) -> String {
    search::sync(&env.ctx_markdown(), &json!({})).expect("sync")
}

fn create_test_markdown_file(env: &TestEnv, filename: &str, content: &str) -> (PathBuf, String) {
    let folder = env.cfg.memory_path.join("tests").join("sync-mtime");
    fs::create_dir_all(&folder).expect("mkdir");
    let file_path = folder.join(filename);
    fs::write(&file_path, content).expect("write fixture");
    let memory_uri = path_to_uri(&file_path, &env.cfg.memory_path).expect("path to uri");
    (file_path, memory_uri)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileContentRow {
    last_indexed: Option<String>,
    file_md5: Option<String>,
    file_size: Option<i64>,
}

fn db_connection(cfg: &Config) -> rusqlite::Connection {
    db::open_connection(&cfg.database_path).expect("db open")
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
        .ok()
}

fn get_mention_count(cfg: &Config, memory_uri: &str, concept: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![normalize_concept(concept), memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .ok()
}

fn tamper_db_delete_mentions(cfg: &Config, memory_uri: &str) {
    db_connection(cfg)
        .execute(
            "DELETE FROM concept_mentions WHERE source_file = ?1",
            params![memory_uri],
        )
        .expect("delete mentions");
}

fn file_mtime(path: &Path) -> SystemTime {
    fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified time")
}

fn file_mtime_rfc3339(path: &Path) -> String {
    DateTime::<Utc>::from(file_mtime(path)).to_rfc3339()
}

fn set_file_mtime_system_time(path: &Path, mtime: SystemTime) {
    set_file_mtime(path, FileTime::from_system_time(mtime)).expect("set file mtime");
}

#[cfg(unix)]
struct PermissionRestore {
    path: PathBuf,
    mode: u32,
}

#[cfg(unix)]
impl Drop for PermissionRestore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode));
    }
}

// T-2.3: RTM FR-14.1 - Ports SyncMtimeOptimizationTests.Sync_WhenMtimeUnchanged_SkipsWithoutRead_AndDoesNotRestoreMentionsOrEdges_OnUnix.
#[cfg(unix)]
#[test]
fn sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "unchanged-mtime.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let original_mtime = file_mtime(&file_path);
    let original_last_indexed = get_file_content_row(&env.cfg, &memory_uri)
        .and_then(|row| row.last_indexed)
        .expect("indexed row should exist after initial sync");
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), Some(1));
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), Some(1));

    tamper_db_delete_mentions(&env.cfg, &memory_uri);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), None);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), None);

    set_file_mtime_system_time(&file_path, original_mtime);

    let original_mode = fs::metadata(&file_path)
        .expect("metadata before chmod")
        .permissions()
        .mode();
    let _restore = PermissionRestore {
        path: file_path.clone(),
        mode: original_mode,
    };
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0)).expect("make file unreadable");

    assert_eq!(
        file_mtime(&file_path),
        original_mtime,
        "making file unreadable must not change mtime"
    );

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "unchanged mtime should be skipped without surfacing a read error: {output}"
    );

    assert_eq!(
        get_file_content_row(&env.cfg, &memory_uri)
            .and_then(|row| row.last_indexed)
            .expect("row should still exist"),
        original_last_indexed,
        "unchanged mtime should leave last_indexed untouched"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_a),
        None,
        "unchanged mtime should skip processing without restoring mentions"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_b),
        None,
        "unchanged mtime should skip processing without restoring mentions"
    );
}

// T-2.3: RTM FR-14.2 - Ports SyncMtimeOptimizationTests.Sync_WhenMtimeBumpedButContentSame_UpdatesLastIndexedOnly_AndDoesNotRestoreMentionsOrEdges.
#[test]
fn sync_when_mtime_bumped_but_content_same_updates_last_indexed_only_and_does_not_restore_mentions()
{
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "same-content-new-mtime.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let before = get_file_content_row(&env.cfg, &memory_uri).expect("initial file_content row");
    tamper_db_delete_mentions(&env.cfg, &memory_uri);

    let desired_mtime = file_mtime(&file_path) + Duration::from_secs(120);
    set_file_mtime_system_time(&file_path, desired_mtime);
    let actual_new_mtime = file_mtime_rfc3339(&file_path);

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "sync should succeed on hash-guard path: {output}"
    );

    let after = get_file_content_row(&env.cfg, &memory_uri).expect("updated file_content row");
    assert_eq!(
        after.last_indexed,
        Some(actual_new_mtime),
        "when content hash matches, sync should update last_indexed to the filesystem mtime"
    );
    assert_eq!(
        after.file_md5, before.file_md5,
        "when content hash matches, sync should preserve file_md5"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_a),
        None,
        "hash guard should not restore mentions"
    );
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_b),
        None,
        "hash guard should not restore mentions"
    );
}

// T-2.3: RTM FR-14.3 - Ports SyncMtimeOptimizationTests.Sync_WhenContentChanged_ReprocessesAndRestoresMentionsAndEdges_IncludingNewConcept.
#[test]
fn sync_when_content_changed_reprocesses_and_restores_mentions_including_new_concept() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let env = isolated_env();

    let concept_a = "sync-mtime-a";
    let concept_b = "sync-mtime-b";
    let concept_c = "sync-mtime-c";
    let (file_path, memory_uri) = create_test_markdown_file(
        &env,
        "content-changed.md",
        &format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]].\n"
        ),
    );

    let baseline = sync_all(&env);
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let before = get_file_content_row(&env.cfg, &memory_uri).expect("initial file_content row");
    let previous_mtime = file_mtime(&file_path);
    tamper_db_delete_mentions(&env.cfg, &memory_uri);
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), None);

    fs::write(
        &file_path,
        format!(
            "# Sync Mtime Optimization Test\n\nReferences [[{concept_a}]] and [[{concept_b}]] and now [[{concept_c}]].\n"
        ),
    )
    .expect("rewrite fixture");
    set_file_mtime_system_time(&file_path, previous_mtime + Duration::from_secs(120));
    let new_mtime = file_mtime_rfc3339(&file_path);

    let output = sync_all(&env);
    assert!(
        output.contains("Sync complete:"),
        "content change should be reprocessed successfully: {output}"
    );

    let after = get_file_content_row(&env.cfg, &memory_uri).expect("updated file_content row");
    assert_eq!(
        after.last_indexed,
        Some(new_mtime),
        "content change should update last_indexed to the filesystem mtime"
    );
    assert_ne!(
        after.file_md5, before.file_md5,
        "content change must update file_md5"
    );
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_a), Some(1));
    assert_eq!(get_mention_count(&env.cfg, &memory_uri, concept_b), Some(1));
    assert_eq!(
        get_mention_count(&env.cfg, &memory_uri, concept_c),
        Some(1),
        "processing must restore mentions and include the new concept"
    );
}
