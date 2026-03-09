use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{normalize_concept, path_to_uri};
use maenifold_core::tools::ToolContext;
use maenifold_core::watchers::{
    format_watcher_error_message, process_path_for_test, start_watcher, stop_watcher,
    watcher_is_running,
};
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
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

fn context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn create_test_file(cfg: &Config, subfolder: &str, content: &str) -> (PathBuf, String) {
    let folder = cfg.memory_path.join(subfolder);
    fs::create_dir_all(&folder).expect("mkdir");
    let file_path = folder.join(format!("note-{}.md", uuid::Uuid::new_v4().simple()));
    fs::write(&file_path, content).expect("write fixture");
    let uri = path_to_uri(&file_path, &cfg.memory_path).expect("memory uri");
    (file_path, uri)
}

fn get_mention_count(cfg: &Config, memory_uri: &str, concept_name: &str) -> Option<i64> {
    db_connection(cfg)
        .query_row(
            "SELECT mention_count FROM concept_mentions WHERE concept_name = ?1 AND source_file = ?2",
            params![concept_name, memory_uri],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("mention count")
}

fn get_file_row(cfg: &Config, memory_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_path FROM file_content WHERE file_path = ?1",
            [memory_uri],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("file row")
}

fn get_file_md5(cfg: &Config, memory_uri: &str) -> Option<String> {
    db_connection(cfg)
        .query_row(
            "SELECT file_md5 FROM file_content WHERE file_path = ?1",
            [memory_uri],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("file md5")
}

fn set_last_indexed(cfg: &Config, memory_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![memory_uri, value],
        )
        .expect("set last_indexed");
}

fn count_rows_like(cfg: &Config, table: &str, pattern: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE file_path LIKE ?1");
    db_connection(cfg)
        .query_row(&sql, [pattern], |row| row.get::<_, i64>(0))
        .expect("row count")
}

fn wait_for(description: &str, timeout: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {description}");
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn watcher_errors_are_formatted_with_expected_prefix() {
    let _guard = TEST_MUTEX.lock();

    let message = format_watcher_error_message("Test watcher error");

    assert_eq!(message, "[FileWatcher] Error: Test watcher error");
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn process_file_created_with_wikilinks_inserts_concept_mentions_into_db() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept_a = format!(
        "incremental-sync-created-a-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_b = format!(
        "incremental-sync-created-b-{}",
        uuid::Uuid::new_v4().simple()
    );
    let content = format!("# Test Note\n\nThis references [[{concept_a}]] and [[{concept_b}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, "tests/incremental-sync-tools", &content);

    process_path_for_test(&ctx, &file_path).expect("process created file");

    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
        Some(1),
        "created file should insert a concept_mention row for the first WikiLink"
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)),
        Some(1),
        "created file should insert a concept_mention row for the second WikiLink"
    );
    assert_eq!(
        get_file_row(&cfg, &memory_uri),
        Some(memory_uri),
        "created file should insert a file_content row"
    );
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn process_file_changed_with_new_content_adds_mention_for_new_concept_and_updates_file_content() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept_a = format!(
        "incremental-sync-changed-a-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_b = format!(
        "incremental-sync-changed-b-{}",
        uuid::Uuid::new_v4().simple()
    );
    let content_v1 = format!("# Test Note\n\nThis references [[{concept_a}]].\n");
    let (file_path, memory_uri) =
        create_test_file(&cfg, "tests/incremental-sync-tools", &content_v1);

    process_path_for_test(&ctx, &file_path).expect("process created file");
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
        Some(1),
        "initial sync should index the original concept"
    );

    let hash_after_create = get_file_md5(&cfg, &memory_uri).expect("stored hash after create");
    set_last_indexed(&cfg, &memory_uri, "2000-01-01T00:00:00+00:00");
    let content_v2 = format!("# Test Note\n\nThis now also references [[{concept_b}]].\n");
    fs::write(&file_path, content_v2).expect("rewrite fixture");

    process_path_for_test(&ctx, &file_path).expect("process changed file");

    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_b)),
        Some(1),
        "changed file should add a mention for the newly introduced WikiLink"
    );
    assert_ne!(
        get_file_md5(&cfg, &memory_uri).expect("stored hash after change"),
        hash_after_create,
        "changed file should update the content hash"
    );
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn process_file_deleted_after_create_removes_file_content_and_mentions_from_db() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let concept = format!("incremental-sync-deleted-{}", uuid::Uuid::new_v4().simple());
    let content = format!("# Test Note\n\nThis references [[{concept}]].\n");
    let (file_path, memory_uri) = create_test_file(&cfg, "tests/incremental-sync-tools", &content);

    process_path_for_test(&ctx, &file_path).expect("process created file");
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept)),
        Some(1),
        "precondition failed: created file should be indexed before deletion"
    );

    fs::remove_file(&file_path).expect("delete fixture");
    process_path_for_test(&ctx, &file_path).expect("process deleted file");

    assert_eq!(
        get_file_row(&cfg, &memory_uri),
        None,
        "deleted file should remove the file_content row"
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept)),
        None,
        "deleted file should remove concept_mention rows"
    );
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn start_watcher_processes_nested_markdown_and_ignores_non_markdown_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let _ = stop_watcher(&ctx, &json!({}));

    start_watcher(&ctx, &json!({"debounceMs": 100})).expect("start watcher");
    let nested = cfg.memory_path.join("tests/incremental-sync-tools/nested");
    fs::create_dir_all(&nested).expect("mkdir nested");
    let ignored_txt = nested.join("ignored.txt");
    let watched_md = nested.join("watched.md");
    fs::write(&ignored_txt, "[[ignored-concept]]\n").expect("write txt");
    fs::write(&watched_md, "# Watched\n\n[[watched-concept]]\n").expect("write markdown");
    let watched_uri = path_to_uri(&watched_md, &cfg.memory_path).expect("watched uri");

    wait_for(
        "watcher to index nested markdown",
        Duration::from_secs(5),
        || get_file_row(&cfg, &watched_uri).is_some(),
    );

    assert_eq!(
        get_file_row(&cfg, &watched_uri),
        Some(watched_uri),
        "watcher should process markdown files in nested memory folders"
    );
    assert_eq!(
        count_rows_like(&cfg, "file_content", "%ignored.txt%"),
        0,
        "watcher should ignore non-markdown files"
    );

    let stop = stop_watcher(&ctx, &json!({})).expect("stop watcher");
    assert_eq!(stop, "Stopped incremental sync watcher");
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn process_file_created_with_path_outside_memory_path_does_not_throw() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let outside_dir = TempDir::new().expect("outside tmp");
    let outside_path = outside_dir.path().join("outside-memory.md");
    fs::write(&outside_path, "# Outside\n\n[[some-concept]]\n").expect("write outside file");

    let result = process_path_for_test(&ctx, &outside_path);

    assert!(
        result.is_ok(),
        "processing a path outside the memory root should be ignored without error: {result:?}"
    );
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn start_watcher_then_stop_watcher_lifecycle_returns_expected_messages() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let _ = stop_watcher(&ctx, &json!({}));

    let start_result = start_watcher(&ctx, &json!({"debounceMs": 250})).expect("start watcher");
    assert!(start_result.contains("Started watching"));
    assert!(start_result.contains(&cfg.memory_path.display().to_string()));

    let second_start_result =
        start_watcher(&ctx, &json!({"debounceMs": 250})).expect("second start watcher");
    assert_eq!(second_start_result, "Watcher already running");
    assert!(
        watcher_is_running(),
        "watcher should remain active after guarded restart"
    );

    let stop_result = stop_watcher(&ctx, &json!({})).expect("stop watcher");
    assert_eq!(stop_result, "Stopped incremental sync watcher");

    let second_stop_result = stop_watcher(&ctx, &json!({})).expect("second stop watcher");
    assert_eq!(second_stop_result, "Watcher not running");
    assert!(
        !watcher_is_running(),
        "watcher should be cleared after stop"
    );
}

// T-COV-001.3: RTM FR-17.5
#[test]
fn debounce_rapid_changes_to_same_path_coalesces_into_final_processing() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let _ = stop_watcher(&ctx, &json!({}));

    let concept_v1 = format!(
        "incremental-sync-debounce-v1-{}",
        uuid::Uuid::new_v4().simple()
    );
    let concept_final = format!(
        "incremental-sync-debounce-final-{}",
        uuid::Uuid::new_v4().simple()
    );
    let (file_path, memory_uri) = create_test_file(
        &cfg,
        "tests/incremental-sync-tools",
        &format!("# Version 1\n\n[[{concept_v1}]]\n"),
    );

    start_watcher(&ctx, &json!({"debounceMs": 150})).expect("start watcher");
    for version in 0..5 {
        fs::write(
            &file_path,
            format!("# Final Version {version}\n\n[[{concept_final}]]\n"),
        )
        .expect("write debounced content");
        thread::sleep(Duration::from_millis(10));
    }

    wait_for(
        "watcher to process final debounced content",
        Duration::from_secs(5),
        || get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_final)) == Some(1),
    );

    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_final)),
        Some(1),
        "debounced watcher processing should index the final file contents"
    );
    assert_eq!(
        get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_v1)),
        None,
        "debounced watcher processing should not leave stale mentions from superseded content"
    );

    let stop = stop_watcher(&ctx, &json!({})).expect("stop watcher");
    assert_eq!(stop, "Stopped incremental sync watcher");
}
