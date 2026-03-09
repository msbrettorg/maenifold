use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

fn semantic_search_output(env: &TestEnv, query: &str) -> String {
    search::search_memories(
        &env.ctx_markdown(),
        &json!({
            "query": query,
            "mode": "Semantic",
            "pageSize": 5,
            "page": 1,
        }),
    )
    .expect("search memories")
}

// T-0.1: RTM NFR-3 - Ports C# VectorSearchTests empty semantic query coverage to guard SearchMemories regressions.
#[test]
fn semantic_search_handles_empty_query_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let output = semantic_search_output(&env, "");

    assert!(
        output.contains("ERROR"),
        "empty query should return a helpful error message: {output}"
    );
}

// T-0.1: RTM NFR-3 - Ports C# VectorSearchTests whitespace-only semantic query coverage to guard SearchMemories regressions.
#[test]
fn semantic_search_handles_whitespace_only_query_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let output = semantic_search_output(&env, "     ");

    assert!(
        output.contains("ERROR"),
        "whitespace-only query should return a helpful error message: {output}"
    );
}

// T-0.1: RTM NFR-3 - Ports C# VectorSearchTests stopword-only semantic query coverage to guard SearchMemories regressions.
#[test]
fn semantic_search_handles_stopword_only_query_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let output = semantic_search_output(&env, "the and of");

    assert!(
        output.contains("ERROR"),
        "stopword-only query should return a helpful error message: {output}"
    );
}
