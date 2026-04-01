use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use serde_json::{Value, json};
use tempfile::TempDir;

// T-12.7: RTM NFR-1
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub const TEST_FOLDER: &str = "concept-graph-tests";
pub const TEST_TAGS: &[&str] = &["ai", "ml", "test"];

pub struct TestEnv {
    pub _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }

    pub fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub fn write_memory_file(env: &TestEnv, title: &str, content: &str) {
    let raw = memory::write_memory(
        &env.ctx_json(),
        &json!({
            "title": title,
            "content": content,
            "folder": TEST_FOLDER,
            "tags": TEST_TAGS,
        }),
    )
    .expect("write memory");
    let value: Value = serde_json::from_str(&raw).expect("write memory json");
    assert_eq!(value["success"], true, "write should succeed: {raw}");
}

pub fn sync_graph(env: &TestEnv) -> String {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "sync contract should announce completion: {output}"
    );
    output
}

pub fn build_context_json(
    env: &TestEnv,
    concept_name: &str,
    depth: usize,
    max_entities: usize,
    include_content: Option<bool>,
) -> Value {
    let mut payload = json!({
        "conceptName": concept_name,
        "depth": depth,
        "maxEntities": max_entities,
    });
    if let Some(include_content) = include_content {
        payload["includeContent"] = Value::Bool(include_content);
    }

    let raw = search::build_context(&env.ctx_json(), &payload).expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

pub fn direct_relations(payload: &Value) -> &[Value] {
    payload["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array")
}

pub fn relation_preview_values(relation: &Value) -> Vec<&str> {
    relation["ContentPreview"]
        .as_object()
        .map(|map| map.values().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
}
