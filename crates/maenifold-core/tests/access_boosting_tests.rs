use chrono::{DateTime, Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "access-boosting-tests";
const READ_SLEEP_MS: u64 = 1_100;

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
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

fn extract_uri(write_result: &str) -> String {
    write_result
        .lines()
        .find_map(|line| {
            line.find("memory://")
                .map(|index| line[index..].trim().to_string())
        })
        .expect("memory:// URI in write result")
}

fn sync_graph(env: &TestEnv) {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
}

fn create_and_sync_test_file(env: &TestEnv, title: &str, content: &str, folder: &str) -> String {
    let result = memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory");
    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected write result: {result}"
    );

    let uri = extract_uri(&result);
    sync_graph(env);
    uri
}

fn read_memory_output(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
}

fn search_memories_json(env: &TestEnv, query: &str, mode: &str, folder: Option<&str>) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": query,
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": folder,
            "tags": Value::Null,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str(&raw).expect("search output json")
}

fn build_context_json(
    env: &TestEnv,
    concept_name: &str,
    depth: usize,
    max_entities: usize,
    include_content: bool,
) -> Value {
    let raw = search::build_context(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "depth": depth,
            "maxEntities": max_entities,
            "includeContent": include_content,
        }),
    )
    .expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

fn get_last_accessed(cfg: &Config, uri: &str) -> Option<DateTime<Utc>> {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    let raw = conn
        .query_row(
            "SELECT last_accessed FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .expect("query last_accessed")
        .flatten()?;

    Some(
        DateTime::parse_from_rfc3339(&raw)
            .expect("valid rfc3339 timestamp")
            .with_timezone(&Utc),
    )
}

fn set_last_accessed(cfg: &Config, uri: &str, last_accessed: &str) {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    conn.execute(
        "UPDATE file_content SET last_accessed = ?1 WHERE file_path = ?2",
        params![last_accessed, uri],
    )
    .expect("update last_accessed");
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn assert_search_contains_title(search_result: &Value, title: &str) {
    let results = search_result["data"]["results"]
        .as_array()
        .expect("results array");
    assert!(
        results
            .iter()
            .any(|entry| entry["title"].as_str() == Some(title)),
        "expected search results to contain {title:?}: {search_result}"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_updates_last_accessed_on_every_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Access Boosting Read Test",
        "Testing [[access-boosting]] behavior for [[ReadMemory]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory should succeed: {read_result}"
    );

    let after_read_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert!(
        after_read_last_accessed.is_some(),
        "last_accessed should be set after ReadMemory"
    );
    assert!(
        after_read_last_accessed > initial_last_accessed,
        "last_accessed should become more recent after ReadMemory"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_updates_last_accessed_on_subsequent_reads() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Multiple Reads Test",
        "Testing [[access-boosting]] with multiple reads for [[decay-weighting]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);

    let first_read_result = read_memory_output(&env, &uri);
    assert!(
        !first_read_result.starts_with("ERROR:"),
        "first ReadMemory should succeed: {first_read_result}"
    );
    let first_read_timestamp = get_last_accessed(&env.cfg, &uri).expect("first read timestamp");

    std::thread::sleep(std::time::Duration::from_millis(READ_SLEEP_MS));

    let second_read_result = read_memory_output(&env, &uri);
    assert!(
        !second_read_result.starts_with("ERROR:"),
        "second ReadMemory should succeed: {second_read_result}"
    );
    let second_read_timestamp = get_last_accessed(&env.cfg, &uri).expect("second read timestamp");

    assert!(
        second_read_timestamp > first_read_timestamp,
        "each ReadMemory call should refresh last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_by_title_updates_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Title Based Access Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[access-boosting]] when reading by [[title]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let read_result = read_memory_output(&env, title);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory by title should succeed: {read_result}"
    );

    let after_read_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert!(
        after_read_last_accessed > initial_last_accessed,
        "reading by title should update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
fn search_memories_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Search Results Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[search-behavior]] for [[access-boosting]]. This contains machine learning keywords.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "search-behavior access-boosting",
        "Hybrid",
        Some(TEST_FOLDER),
    );
    assert_search_contains_title(&search_result, title);

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "SearchMemories should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
#[ignore = "Semantic search parity is ONNX-dependent"]
fn search_memories_semantic_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Semantic Search Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[semantic-search]] and [[vector-embeddings]] behavior.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "semantic search vector",
        "Semantic",
        Some(TEST_FOLDER),
    );
    assert_search_contains_title(&search_result, title);

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "Semantic SearchMemories should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
#[test]
fn search_memories_fulltext_mode_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "FullText Search Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[fulltext-search]] and [[keyword-matching]] behavior.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let search_result = search_memories_json(
        &env,
        "fulltext keyword matching",
        "FullText",
        Some(TEST_FOLDER),
    );
    assert_search_contains_title(&search_result, title);

    let after_search_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search_last_accessed, initial_last_accessed,
        "FullText SearchMemories should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
#[test]
fn build_context_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri1 = create_and_sync_test_file(
        &env,
        "BuildContext Test File 1",
        "Testing [[build-context]] and [[graph-traversal]] concepts.",
        TEST_FOLDER,
    );
    let uri2 = create_and_sync_test_file(
        &env,
        "BuildContext Test File 2",
        "More content about [[build-context]] and [[concept-relationships]].",
        TEST_FOLDER,
    );

    let read_one = read_memory_output(&env, &uri1);
    let read_two = read_memory_output(&env, &uri2);
    assert!(!read_one.starts_with("ERROR:"), "unexpected: {read_one}");
    assert!(!read_two.starts_with("ERROR:"), "unexpected: {read_two}");

    let baseline_one = rfc3339_days_ago(5);
    let baseline_two = rfc3339_days_ago(4);
    set_last_accessed(&env.cfg, &uri1, &baseline_one);
    set_last_accessed(&env.cfg, &uri2, &baseline_two);

    let initial_last_accessed1 = get_last_accessed(&env.cfg, &uri1);
    let initial_last_accessed2 = get_last_accessed(&env.cfg, &uri2);

    let context_result = build_context_json(&env, "build-context", 2, 20, true);
    let direct_relations = context_result["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array");
    assert!(
        !direct_relations.is_empty(),
        "BuildContext should find related concepts: {context_result}"
    );

    let after_build_context_last_accessed1 = get_last_accessed(&env.cfg, &uri1);
    let after_build_context_last_accessed2 = get_last_accessed(&env.cfg, &uri2);

    assert_eq!(
        after_build_context_last_accessed1, initial_last_accessed1,
        "BuildContext should not update last_accessed for the first referenced file"
    );
    assert_eq!(
        after_build_context_last_accessed2, initial_last_accessed2,
        "BuildContext should not update last_accessed for the second referenced file"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
#[test]
fn build_context_with_content_preview_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Content Preview Test",
        "Testing [[content-preview]] in [[BuildContext]] with [[access-boosting]].",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let context_result = build_context_json(&env, "content-preview", 1, 10, true);
    assert_eq!(
        context_result["data"]["ConceptName"].as_str(),
        Some("content-preview"),
        "unexpected BuildContext output: {context_result}"
    );

    let after_context_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_context_last_accessed, initial_last_accessed,
        "BuildContext with includeContent=true should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
#[test]
fn build_context_at_various_depths_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Depth Test File",
        "Testing [[depth-traversal]] with [[graph-depth]] concepts.",
        TEST_FOLDER,
    );

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let context_result = build_context_json(&env, "depth-traversal", 3, 50, false);
    assert_eq!(
        context_result["data"]["Depth"].as_u64(),
        Some(3),
        "unexpected BuildContext output: {context_result}"
    );

    let after_context_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_context_last_accessed, initial_last_accessed,
        "BuildContext at depth=3 should not update last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1, NFR-7.6.2, NFR-7.6.3
#[test]
fn only_read_memory_updates_last_accessed_not_search_or_build_context() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Combined Behavior Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[combined-access]] patterns for [[decay-weighting]] and [[boosting]].",
        TEST_FOLDER,
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);

    let first_read_result = read_memory_output(&env, &uri);
    assert!(
        !first_read_result.starts_with("ERROR:"),
        "first read should succeed: {first_read_result}"
    );
    let after_first_read = get_last_accessed(&env.cfg, &uri).expect("last_accessed after read");

    let search_result =
        search_memories_json(&env, "combined-access decay", "Hybrid", Some(TEST_FOLDER));
    assert_search_contains_title(&search_result, title);
    let after_search = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_search,
        Some(after_first_read),
        "SearchMemories should not update last_accessed"
    );

    let context_result = build_context_json(&env, "combined-access", 2, 20, true);
    assert_eq!(
        context_result["data"]["ConceptName"].as_str(),
        Some("combined-access"),
        "unexpected BuildContext output: {context_result}"
    );
    let after_build_context = get_last_accessed(&env.cfg, &uri);
    assert_eq!(
        after_build_context,
        Some(after_first_read),
        "BuildContext should not update last_accessed"
    );

    std::thread::sleep(std::time::Duration::from_millis(READ_SLEEP_MS));

    let second_read_result = read_memory_output(&env, &uri);
    assert!(
        !second_read_result.starts_with("ERROR:"),
        "second read should succeed: {second_read_result}"
    );
    let after_second_read =
        get_last_accessed(&env.cfg, &uri).expect("last_accessed after second read");
    assert!(
        after_second_read > after_first_read,
        "a subsequent ReadMemory should update last_accessed"
    );
}
