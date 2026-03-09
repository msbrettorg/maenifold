use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "search-tools-tests";

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
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

fn create_test_file_with_content(
    env: &TestEnv,
    relative_folder: &str,
    title: &str,
    content: &str,
    tags: &[&str],
) {
    let folder = format!("{TEST_FOLDER}/{relative_folder}");
    let result = memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
            "tags": tags,
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected write result: {result}"
    );
}

fn create_test_data_with_known_scores(env: &TestEnv) {
    create_test_file_with_content(
        env,
        "high-relevance",
        "Machine Learning Tutorial",
        "This is a comprehensive guide to [[machine learning]] algorithms and [[deep learning]] neural networks. Machine learning is the core topic of this document with detailed explanations of machine learning concepts.",
        &["ml", "tutorial", "ai"],
    );

    create_test_file_with_content(
        env,
        "medium-relevance",
        "Artificial Intelligence Overview",
        "This document covers [[artificial intelligence]] concepts and some [[machine learning]] basics. AI includes various techniques including some machine learning approaches.",
        &["ai", "overview"],
    );

    create_test_file_with_content(
        env,
        "low-relevance",
        "Data Analysis Methods",
        "This document discusses [[data analysis]] and [[statistics]] with brief mention of learning algorithms. Statistical methods for analyzing data with minimal machine references.",
        &["data", "stats"],
    );

    create_test_file_with_content(
        env,
        "low-relevance",
        "Cooking Recipes",
        "This document contains [[cooking]] recipes and [[food]] preparation techniques. Kitchen equipment and cooking methods for preparing meals.",
        &["cooking", "food"],
    );

    let sync_output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        sync_output.contains("Sync complete:"),
        "unexpected sync output: {sync_output}"
    );
}

fn search_output(env: &TestEnv, mode: &str, min_score: f64) -> String {
    search::search_memories(
        &env.ctx_markdown(),
        &json!({
            "query": "machine learning",
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "tags": serde_json::Value::Null,
            "minScore": min_score,
        }),
    )
    .expect("search memories")
}

// T-0.1: RTM NFR-3 - Ports C# SearchToolsTests minScore=0.0 coverage to guard SearchMemories regressions.
#[test]
fn min_score_zero_returns_all_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    create_test_data_with_known_scores(&env);

    let result = search_output(&env, "Hybrid", 0.0);

    assert!(
        result.contains("Machine Learning Tutorial"),
        "should include high relevance document: {result}"
    );
    assert!(
        result.contains("Artificial Intelligence Overview"),
        "should include medium relevance document: {result}"
    );
    assert!(
        result.contains("Found"),
        "should render search results: {result}"
    );
    assert!(
        !result.contains("ERROR"),
        "should not return an error: {result}"
    );
}

// T-0.1: RTM NFR-3 - Ports C# SearchToolsTests minScore=1.0 coverage to guard SearchMemories regressions.
#[test]
fn min_score_one_returns_only_perfect_matches() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    create_test_data_with_known_scores(&env);

    let result = search_output(&env, "Hybrid", 1.0);

    assert!(
        !result.contains("ERROR"),
        "should not return an error: {result}"
    );
    if result.contains("Machine Learning Tutorial") {
        assert!(
            !result.contains("Cooking Recipes"),
            "should not include unrelated low-score content: {result}"
        );
    }
}

// T-0.1: RTM NFR-3 - Ports C# SearchToolsTests cross-mode minScore coverage to guard SearchMemories regressions.
#[test]
fn all_search_modes_respect_min_score() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    create_test_data_with_known_scores(&env);

    let semantic_result = search_output(&env, "Semantic", 0.5);
    assert!(
        semantic_result.contains("Semantic Search"),
        "should indicate semantic search mode: {semantic_result}"
    );
    assert!(
        !semantic_result.contains("ERROR"),
        "semantic search should not return an error: {semantic_result}"
    );

    let full_text_result = search_output(&env, "FullText", 0.5);
    assert!(
        full_text_result.contains("Full-Text Search"),
        "should indicate full-text search mode: {full_text_result}"
    );
    assert!(
        !full_text_result.contains("ERROR"),
        "full-text search should not return an error: {full_text_result}"
    );

    let hybrid_result = search_output(&env, "Hybrid", 0.5);
    assert!(
        hybrid_result.contains("Hybrid Search"),
        "should indicate hybrid search mode: {hybrid_result}"
    );
    assert!(
        !hybrid_result.contains("ERROR"),
        "hybrid search should not return an error: {hybrid_result}"
    );

    assert!(
        !semantic_result.contains("Cooking Recipes"),
        "semantic search should filter out unrelated content with moderate minScore: {semantic_result}"
    );
    assert!(
        !full_text_result.contains("Cooking Recipes"),
        "full-text search should filter out unrelated content with moderate minScore: {full_text_result}"
    );
    assert!(
        !hybrid_result.contains("Cooking Recipes"),
        "hybrid search should filter out unrelated content with moderate minScore: {hybrid_result}"
    );
}
