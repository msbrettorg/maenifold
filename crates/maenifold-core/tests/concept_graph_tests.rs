use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "concept-graph-tests";
const TEST_TAGS: &[&str] = &["ai", "ml", "test"];

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }

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

fn write_memory_file(env: &TestEnv, title: &str, content: &str) {
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

fn sync_graph(env: &TestEnv) -> String {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "sync contract should announce completion: {output}"
    );
    output
}

fn build_context_json(
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

fn direct_relations<'a>(payload: &'a Value) -> &'a [Value] {
    payload["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array")
}

fn relation_preview_values(relation: &Value) -> Vec<&str> {
    relation["ContentPreview"]
        .as_object()
        .map(|map| map.values().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs ConceptGraphSmokeTest.
#[test]
fn concept_graph_smoke_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "AI Concepts Test",
        "This document explores [[Artificial Intelligence]] and its relationship to [[Machine Learning]] and [[Deep Learning]].\n\n[[Deep Learning]] is a subset of [[Machine Learning]], which itself is a subset of [[Artificial Intelligence]].",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Machine Learning", 2, 20, Some(false));
    let direct = direct_relations(&context);

    assert!(
        !direct.is_empty(),
        "expected BuildContext to return related concepts: {context}"
    );
    assert!(
        direct.iter().any(|relation| {
            matches!(
                relation["Name"].as_str(),
                Some("artificial-intelligence" | "deep-learning")
            )
        }),
        "expected direct relation to AI or Deep Learning: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextWithoutContentTest.
#[test]
fn build_context_without_content_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Machine Learning Concepts for Content Test",
        "This document discusses [[Machine Learning]] and its applications in modern systems.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Machine Learning", 2, 20, Some(false));
    let direct = direct_relations(&context);

    assert!(
        direct
            .iter()
            .all(|relation| relation_preview_values(relation).is_empty()),
        "includeContent=false should not populate previews: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextWithContentTest.
#[test]
fn build_context_with_content_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Deep Learning Guide",
        "Comprehensive guide to [[Deep Learning]] using [[Neural Networks]]. [[Deep Learning]] is part of [[Machine Learning]].",
    );
    write_memory_file(
        &env,
        "Machine Learning Overview",
        "Overview of [[Machine Learning]] and [[Deep Learning]] in [[AI]]. [[Machine Learning]] uses [[Neural Networks]].",
    );
    write_memory_file(
        &env,
        "Neural Networks Tutorial",
        "Tutorial on [[Neural Networks]] for [[Deep Learning]]. [[Neural Networks]] power modern [[AI]].",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Deep Learning", 2, 20, Some(true));
    let direct = direct_relations(&context);
    let preview_values = direct
        .iter()
        .flat_map(relation_preview_values)
        .collect::<Vec<_>>();

    assert!(
        !direct.is_empty(),
        "should have direct relations from shared concepts: {context}"
    );
    assert!(
        !preview_values.is_empty(),
        "includeContent=true should populate content previews: {context}"
    );
    assert!(
        preview_values.iter().any(|preview| {
            preview.contains("Comprehensive guide")
                || preview.contains("Overview of [[Machine Learning]]")
                || preview.contains("Tutorial on [[Neural Networks]]")
        }),
        "expected previews from contributing files: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextIncludeContentDefaultTest.
#[test]
fn build_context_include_content_default_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "AI Overview for Default Test",
        "Overview of [[Artificial Intelligence]] and [[Modern Systems]] in production environments.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "Artificial Intelligence", 2, 20, None);
    let direct = direct_relations(&context);

    assert!(
        !direct.is_empty(),
        "default-path test should include at least one relation: {context}"
    );
    assert!(
        direct
            .iter()
            .all(|relation| relation_preview_values(relation).is_empty()),
        "default includeContent should remain false: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextExtractsSectionWithConceptTest.
#[test]
fn build_context_extracts_section_with_concept_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Authentication System Overview",
        "# Authentication System Overview\n\n## Introduction\n\nThis document describes our authentication system architecture.\n\n## Security\n\nThe system uses [[authentication]] via JWT tokens. All requests must include valid [[authentication]] headers.\n\n## Configuration\n\nSee config.yml for settings.",
    );
    write_memory_file(
        &env,
        "JWT Implementation",
        "# JWT Implementation\n\n## Overview\n\nJWT tokens are used for [[authentication]]. This document covers [[JWT]] implementation details.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "authentication", 1, 20, Some(true));
    let direct = direct_relations(&context);
    assert!(
        !direct.is_empty(),
        "should have direct relations: {context}"
    );

    let preview_values = direct
        .iter()
        .flat_map(relation_preview_values)
        .collect::<Vec<_>>();

    assert!(
        preview_values
            .iter()
            .any(|preview| { preview.contains("## Security") || preview.contains("JWT tokens") }),
        "preview should contain the section where authentication is mentioned: {context}"
    );
    assert!(
        preview_values
            .iter()
            .all(|preview| !preview.starts_with("## Introduction")),
        "preview should not start at the intro section when the concept is in a later section: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs ExtractSectionWithConceptEdgeCasesTest.
#[test]
fn extract_section_with_concept_edge_cases_test() {
    let _guard = TEST_MUTEX.lock();

    let content1 = "# Title\n## Intro\nSome intro.\n## Security\nUsing [[authentication]] here.\n## End\nThe end.";
    let result1 = search::extract_section_with_concept(content1, "authentication", 500);
    assert!(
        result1.contains("## Security"),
        "should extract Security section: {result1}"
    );
    assert!(
        result1.contains("[[authentication]]"),
        "should include the concept mention: {result1}"
    );

    let content2 = "# Title\n## Intro\nMentions [[auth]].\n## Details\nMore [[auth]] info.";
    let result2 = search::extract_section_with_concept(content2, "auth", 500);
    assert!(
        result2.contains("## Intro"),
        "should extract first section with mention: {result2}"
    );

    let content3 = "# Title\nThis uses [[jwt]] tokens.\nMore content here.";
    let result3 = search::extract_section_with_concept(content3, "jwt", 500);
    assert!(
        result3.contains("[[jwt]]"),
        "should extract H1 section: {result3}"
    );

    let content4 = "# Title\n## Section\nNo concept here.";
    let result4 = search::extract_section_with_concept(content4, "nonexistent", 500);
    assert!(
        result4.contains("Title"),
        "should fallback to a preview from the start of the document: {result4}"
    );

    let content5 = "# Title\n## Usage\nPlain authentication without brackets.";
    let result5 = search::extract_section_with_concept(content5, "authentication", 500);
    assert!(
        result5.contains("## Usage"),
        "should match plain text: {result5}"
    );
    assert!(
        result5.contains("authentication"),
        "should include the plain-text mention: {result5}"
    );

    let long_content = format!("# Title\n## Long\nUsing [[test]]. {}", "x".repeat(1000));
    let result6 = search::extract_section_with_concept(&long_content, "test", 100);
    assert!(
        result6.len() <= 200,
        "long sections should be truncated to a compact preview: {}",
        result6.len()
    );

    let result7 = search::extract_section_with_concept("", "test", 500);
    assert!(
        result7.is_empty(),
        "empty content should return an empty preview"
    );

    let result8 = search::extract_section_with_concept("   \n  \n", "test", 500);
    assert!(
        result8.trim().is_empty(),
        "whitespace-only content should not produce a non-empty preview: {result8:?}"
    );
}
