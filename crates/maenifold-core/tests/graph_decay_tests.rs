use chrono::{DateTime, Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory::{self, compute_decay_weight};
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "graph-decay-weighting-tests";

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
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn memory_uri_from_relative(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn create_test_file_with_age(
    cfg: &Config,
    relative_path: &str,
    title: &str,
    content: &str,
    days_ago: i64,
) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    let parent = file_path.parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create test dirs");

    let created_iso = rfc3339_days_ago(days_ago);
    let full_content = format!(
        "---\n\
         title: {title}\n\
         type: memory\n\
         status: saved\n\
         created: {created_iso}\n\
         modified: {created_iso}\n\
         ---\n\n\
         # {title}\n\n\
         {content}\n"
    );
    fs::write(&file_path, full_content).expect("write memory file");

    memory_uri_from_relative(relative_path)
}

fn sync_graph(env: &TestEnv) {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
}

fn search_memories_json(env: &TestEnv, query: &str, mode: &str) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": query,
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str(&raw).expect("search memories json")
}

fn build_context_json(env: &TestEnv, concept_name: &str) -> Value {
    let raw = search::build_context(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "depth": 1,
            "maxEntities": 20,
            "includeContent": false,
        }),
    )
    .expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

fn find_similar_concepts_markdown(env: &TestEnv, concept_name: &str) -> String {
    search::find_similar_concepts(
        &env.ctx_markdown(),
        &json!({
            "conceptName": concept_name,
            "maxResults": 20,
        }),
    )
    .expect("find similar concepts")
}

fn read_memory_markdown(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
}

fn find_result<'a>(response: &'a Value, title: &str) -> &'a Value {
    response["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .find(|entry| entry["title"].as_str() == Some(title))
        .unwrap_or_else(|| panic!("missing result for {title:?}: {response}"))
}

fn title_index(response: &Value, title: &str) -> usize {
    response["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .position(|entry| entry["title"].as_str() == Some(title))
        .unwrap_or_else(|| panic!("missing result for {title:?}: {response}"))
}

fn get_last_accessed(cfg: &Config, uri: &str) -> Option<DateTime<Utc>> {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let raw = connection
        .query_row(
            "SELECT last_accessed FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("query last_accessed")?;

    Some(
        DateTime::parse_from_rfc3339(&raw)
            .expect("valid rfc3339 timestamp")
            .with_timezone(&Utc),
    )
}

fn decay_weight_for_uri(cfg: &Config, uri: &str) -> f64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let (last_accessed, status) = connection
        .query_row(
            "SELECT last_accessed, status FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .expect("query decay metadata");

    compute_decay_weight(uri, last_accessed.as_deref(), cfg, status.as_deref())
}

fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_newer_content_ranks_higher_when_scores_are_equal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content = "This document discusses [[machine-learning]] algorithms and [[deep-learning]] neural networks. The primary focus is on supervised [[machine-learning]] techniques for classification tasks.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-ml-doc.md"),
        "Old Machine Learning Doc",
        identical_content,
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-ml-doc.md"),
        "New Machine Learning Doc",
        identical_content,
        1,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "machine learning deep learning", "Hybrid");

    assert!(
        search_result["success"].as_bool() == Some(true),
        "{search_result}"
    );
    assert!(
        title_index(&search_result, "New Machine Learning Doc")
            < title_index(&search_result, "Old Machine Learning Doc"),
        "newer document should rank ahead of older document: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_semantic_mode_applies_decay_weighting() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content =
        "Understanding [[vector-embeddings]] for [[semantic-search]] in AI systems.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-embeddings.md"),
        "Old Embeddings Doc",
        identical_content,
        45,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-embeddings.md"),
        "New Embeddings Doc",
        identical_content,
        2,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "vector embeddings semantic search", "Semantic");

    assert!(
        title_index(&search_result, "New Embeddings Doc")
            < title_index(&search_result, "Old Embeddings Doc"),
        "newer document should rank ahead in semantic mode: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn search_memories_fulltext_mode_applies_decay_weighting() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content =
        "This covers [[authentication]] patterns and [[jwt]] token validation strategies.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth.md"),
        "Old Authentication Doc",
        identical_content,
        50,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-auth.md"),
        "New Authentication Doc",
        identical_content,
        3,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "authentication jwt token", "FullText");

    assert!(
        title_index(&search_result, "New Authentication Doc")
            < title_index(&search_result, "Old Authentication Doc"),
        "newer document should rank ahead in full-text mode: {search_result}"
    );
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn build_context_newer_related_concepts_rank_higher() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-api-patterns.md"),
        "Old API Patterns",
        "This discusses [[api-design]] with [[rest-architecture]] and [[old-pattern]] legacy approaches.",
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-api-patterns.md"),
        "New API Patterns",
        "Modern [[api-design]] using [[graphql]] and [[new-pattern]] contemporary approaches.",
        2,
    );

    sync_graph(&env);

    let context_result = build_context_json(&env, "api-design");
    let relations = context_result["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array");

    let new_index = relations
        .iter()
        .position(|entry| entry["Name"].as_str() == Some("new-pattern"));
    let old_index = relations
        .iter()
        .position(|entry| entry["Name"].as_str() == Some("old-pattern"));

    if let (Some(new_index), Some(old_index)) = (new_index, old_index) {
        assert!(
            new_index < old_index,
            "newer related concept should rank ahead of older one: {context_result}"
        );
    } else {
        panic!("expected both new-pattern and old-pattern in build context: {context_result}");
    }
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn find_similar_concepts_newer_concepts_rank_higher() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-framework.md"),
        "Old Framework Doc",
        "This covers [[legacy-framework]] for [[software-architecture]] applications.",
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-framework.md"),
        "New Framework Doc",
        "This covers [[modern-framework]] for [[software-architecture]] applications.",
        2,
    );

    sync_graph(&env);

    let similar_result = find_similar_concepts_markdown(&env, "software-architecture");

    assert!(
        !similar_result.contains("ERROR"),
        "FindSimilarConcepts should succeed: {similar_result}"
    );

    let modern_index = similar_result.find("modern-framework");
    let legacy_index = similar_result.find("legacy-framework");
    if let (Some(modern_index), Some(legacy_index)) = (modern_index, legacy_index) {
        assert!(
            modern_index < legacy_index,
            "newer framework concept should rank ahead of legacy concept: {similar_result}"
        );
    } else {
        panic!("expected both framework concepts in similar concepts output: {similar_result}");
    }
}

// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
#[test]
fn read_memory_bypasses_decay_and_returns_full_content_regardless_of_age() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/ancient-doc.md"),
        "Ancient Document",
        "This is the full content with [[decay-testing]] concepts that should be fully retrievable.",
        365,
    );

    let uri = format!("memory://{TEST_FOLDER}/ancient-doc");
    let read_result = read_memory_markdown(&env, &uri);

    assert!(!read_result.starts_with("ERROR:"), "{read_result}");
    assert!(read_result.contains("Ancient Document"), "{read_result}");
    assert!(read_result.contains("[[decay-testing]]"), "{read_result}");
    assert!(read_result.contains("fully retrievable"), "{read_result}");
}

// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
#[test]
fn read_memory_decayed_content_remains_fully_retrievable() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let decayed_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/decayed-file.md"),
        "Decayed File",
        "Content about [[retrieval-testing]] for verification.",
        90,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/fresh-file.md"),
        "Fresh File",
        "Content about [[retrieval-testing]] for verification.",
        1,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "retrieval testing", "Hybrid");
    let read_result = read_memory_markdown(&env, &decayed_uri);

    assert!(read_result.contains("Decayed File"), "{read_result}");
    assert!(
        read_result.contains("[[retrieval-testing]]"),
        "{read_result}"
    );
    assert!(
        find_result(&search_result, "Decayed File").is_object(),
        "decayed file should remain searchable: {search_result}"
    );
    assert!(
        find_result(&search_result, "Fresh File").is_object(),
        "fresh file should remain searchable: {search_result}"
    );
}

// T-GRAPH-DECAY-002.1: RTM NFR-7.6.1
#[test]
fn get_decay_weight_after_read_memory_returns_higher_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/access-boost-test.md"),
        "Access Boost Test",
        "This document discusses [[access-boosting]] and [[decay-reset]] behavior.",
        60,
    );

    sync_graph(&env);

    let weight_before_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_before_access < 0.8,
        "60-day-old file should have significant decay before access boosting: {weight_before_access}"
    );

    let read_result = read_memory_markdown(&env, &uri);
    assert!(!read_result.starts_with("ERROR:"), "{read_result}");

    let weight_after_access = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        weight_after_access > weight_before_access,
        "weight should increase after ReadMemory: before={weight_before_access}, after={weight_after_access}"
    );
    assert!(
        weight_after_access >= 0.95,
        "recently read file should have near-full weight: {weight_after_access}"
    );
}

// T-GRAPH-DECAY-002.1: RTM NFR-7.6.1
#[test]
fn get_decay_weight_after_search_memories_weight_is_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/search-no-boost.md"),
        "Search No Boost",
        "This document discusses [[search-no-boost]] and [[passive-access]] patterns.",
        60,
    );

    sync_graph(&env);

    let initial_last_accessed = get_last_accessed(&env.cfg, &uri).expect("initial last_accessed");
    let weight_before = decay_weight_for_uri(&env.cfg, &uri);

    let _search_result = search_memories_json(&env, "search no boost passive access", "Hybrid");

    let after_last_accessed =
        get_last_accessed(&env.cfg, &uri).expect("last_accessed after search");
    let weight_after = decay_weight_for_uri(&env.cfg, &uri);

    assert_eq!(
        after_last_accessed, initial_last_accessed,
        "SearchMemories should not update last_accessed"
    );
    assert_approx_eq(weight_after, weight_before, 0.000_001);
}

// T-GRAPH-DECAY-001.1: RTM NFR-7.5.2
#[test]
fn search_memories_files_within_grace_period_have_equal_ranking_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let identical_content = "Discussing [[grace-period]] behavior for [[decay-weighting]] tests.";
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/day1-file.md"),
        "Day 1 File",
        identical_content,
        1,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/day10-file.md"),
        "Day 10 File",
        identical_content,
        10,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "grace period decay weighting", "Hybrid");
    let day1 = find_result(&search_result, "Day 1 File");
    let day10 = find_result(&search_result, "Day 10 File");

    assert_approx_eq(
        day1["decayWeight"].as_f64().expect("day1 decay weight"),
        1.0,
        0.000_001,
    );
    assert_approx_eq(
        day10["decayWeight"].as_f64().expect("day10 decay weight"),
        1.0,
        0.000_001,
    );
}
