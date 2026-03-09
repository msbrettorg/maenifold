use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
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

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, content).expect("write fixture");
}

fn file_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

fn baseline_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("baseline sync")
}

fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn fetch_last_indexed(cfg: &Config, file_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT last_indexed FROM file_content WHERE file_path = ?1",
            [file_uri],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

fn fetch_file_md5(cfg: &Config, file_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_md5 FROM file_content WHERE file_path = ?1",
            [file_uri],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

fn set_last_indexed(cfg: &Config, file_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, value],
        )
        .expect("set last_indexed");
}

fn concept_names(cfg: &Config) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare("SELECT concept_name FROM concepts ORDER BY concept_name")
        .expect("prepare concepts");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query concepts")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concepts")
}

fn file_concepts(cfg: &Config, file_uri: &str) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare(
            "SELECT concept_name FROM concept_mentions WHERE source_file = ?1 ORDER BY concept_name",
        )
        .expect("prepare concept_mentions");
    statement
        .query_map([file_uri], |row| row.get::<_, String>(0))
        .expect("query concept_mentions")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concept_mentions")
}

fn concept_sources(cfg: &Config, concept_name: &str) -> BTreeSet<String> {
    let connection = db_connection(cfg);
    let mut statement = connection
        .prepare(
            "SELECT source_file FROM concept_mentions WHERE concept_name = ?1 ORDER BY source_file",
        )
        .expect("prepare concept source query");
    statement
        .query_map([concept_name], |row| row.get::<_, String>(0))
        .expect("query concept sources")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concept sources")
}

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_processes_a_new_file_correctly() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "notes/baseline.md",
        "# Baseline\n[[seed]] [[shared]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let baseline_uri = file_uri("notes/baseline.md");
    let baseline_last_indexed = fetch_last_indexed(&cfg, &baseline_uri).expect("baseline indexed");

    write_memory_fixture(&cfg, "notes/new-file.md", "# New\n[[fresh]] [[shared]]\n");
    let output = sync_one(&cfg, "notes/new-file.md");

    assert!(
        output.contains("Synced memory://notes/new-file"),
        "new file should be processed: {output}"
    );
    assert!(
        output.contains("2 concept mentions updated"),
        "expected fresh file mention count in output: {output}"
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from([
            "fresh".to_string(),
            "seed".to_string(),
            "shared".to_string()
        ])
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &baseline_uri),
        Some(baseline_last_indexed),
        "unchanged baseline file should not be reprocessed"
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_skips_unchanged_file_with_mtime_guard() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "notes/mtime-guard.md", "# Note\n[[alpha]] [[beta]]\n");

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/mtime-guard.md");
    let before_last_indexed = fetch_last_indexed(&cfg, &target_uri).expect("indexed");
    let before_concepts = concept_names(&cfg);

    let output = sync_one(&cfg, "notes/mtime-guard.md");

    assert!(
        output.contains("mtime guard"),
        "unchanged file should skip on mtime guard: {output}"
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &target_uri),
        Some(before_last_indexed),
        "mtime guard should not update last_indexed"
    );
    assert_eq!(
        concept_names(&cfg),
        before_concepts,
        "mtime guard should leave graph state unchanged"
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_skips_file_with_same_hash() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let content = "# Hash Guard\n[[alpha]] [[shared]]\n";
    write_memory_fixture(&cfg, "notes/hash-guard.md", content);

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/hash-guard.md");
    let old_timestamp = "2000-01-01T00:00:00+00:00";
    let original_hash = fetch_file_md5(&cfg, &target_uri).expect("stored hash");
    let before_concepts = concept_names(&cfg);
    set_last_indexed(&cfg, &target_uri, old_timestamp);
    write_memory_fixture(&cfg, "notes/hash-guard.md", content);

    let output = sync_one(&cfg, "notes/hash-guard.md");

    assert!(
        output.contains("hash guard"),
        "same content should skip on hash guard: {output}"
    );
    let updated_last_indexed = fetch_last_indexed(&cfg, &target_uri).expect("updated indexed_at");
    assert_ne!(
        updated_last_indexed, old_timestamp,
        "hash guard should refresh last_indexed after re-check"
    );
    assert_eq!(
        fetch_file_md5(&cfg, &target_uri),
        Some(original_hash),
        "hash guard should preserve stored checksum"
    );
    assert_eq!(
        concept_names(&cfg),
        before_concepts,
        "hash guard should not rebuild unrelated concepts"
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn sync_single_file_updates_concepts_when_content_changes() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "notes/target.md", "# Target\n[[alpha]] [[shared]]\n");
    write_memory_fixture(
        &cfg,
        "notes/untouched.md",
        "# Untouched\n[[shared]] [[stable]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    let target_uri = file_uri("notes/target.md");
    let untouched_uri = file_uri("notes/untouched.md");
    let untouched_last_indexed =
        fetch_last_indexed(&cfg, &untouched_uri).expect("untouched last_indexed");
    set_last_indexed(&cfg, &target_uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(&cfg, "notes/target.md", "# Target\n[[beta]] [[shared]]\n");

    let output = sync_one(&cfg, "notes/target.md");

    assert!(
        output.contains("Synced memory://notes/target"),
        "changed file should be processed: {output}"
    );
    assert_eq!(
        file_concepts(&cfg, &target_uri),
        BTreeSet::from(["beta".to_string(), "shared".to_string()])
    );
    assert!(
        concept_sources(&cfg, "alpha").is_empty(),
        "alpha should have no remaining mentions: {:?}",
        concept_sources(&cfg, "alpha")
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from([
            "beta".to_string(),
            "shared".to_string(),
            "stable".to_string()
        ]),
        "old concept should be removed and untouched concepts preserved"
    );
    assert_eq!(
        fetch_last_indexed(&cfg, &untouched_uri),
        Some(untouched_last_indexed),
        "unchanged file should not be reprocessed during single-file sync"
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn deleted_files_remove_their_concepts_from_the_graph() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "notes/deleted.md",
        "# Deleted\n[[alpha]] [[shared]]\n",
    );
    write_memory_fixture(
        &cfg,
        "notes/survivor.md",
        "# Survivor\n[[shared]] [[stable]]\n",
    );

    let baseline = baseline_sync();
    assert!(
        baseline.contains("Sync complete:"),
        "baseline sync should succeed: {baseline}"
    );

    fs::remove_file(cfg.memory_path.join("notes/deleted.md")).expect("delete target file");
    let output = sync_one(&cfg, "notes/deleted.md");

    assert!(
        output.contains("removed stale index entry"),
        "deleted file should clear indexed concepts: {output}"
    );
    assert_eq!(
        concept_names(&cfg),
        BTreeSet::from(["shared".to_string(), "stable".to_string()]),
        "deleted file's unique concepts should be removed"
    );
    assert!(
        file_concepts(&cfg, &file_uri("notes/deleted.md")).is_empty(),
        "deleted file should have no remaining concept mentions"
    );
    assert!(
        concept_sources(&cfg, "alpha").is_empty(),
        "deleted alpha concept should have no remaining mentions: {:?}",
        concept_sources(&cfg, "alpha")
    );
}

// T-5.7: RTM FR-5.7
#[test]
fn watcher_start_stop_lifecycle_contract_still_holds() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let start = ToolRegistry::global()
        .try_invoke(
            "StartWatcher",
            &json!({"debounceMs": 250}),
            OutputFormat::Markdown,
        )
        .expect("start watcher");
    assert!(
        start.starts_with("Started watching "),
        "unexpected start output: {start}"
    );
    assert!(
        start.contains(" for incremental sync (debounce: 250ms)"),
        "unexpected start output: {start}"
    );

    let stop = ToolRegistry::global()
        .try_invoke("StopWatcher", &json!({}), OutputFormat::Markdown)
        .expect("stop watcher");
    assert!(
        stop == "Stopped watcher" || stop == "Watcher not running",
        "unexpected stop output: {stop}"
    );
}
