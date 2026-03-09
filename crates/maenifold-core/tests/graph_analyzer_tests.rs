use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::params;
use serde_json::json;
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

fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

fn seed_graph(cfg: &Config) {
    let connection = db::open_connection(&cfg.database_path).expect("db open");

    for concept in [
        "concept-a",
        "concept-b",
        "concept-c",
        "concept-d",
        "concept-e",
        "isolated-concept",
    ] {
        insert_concept(&connection, concept);
    }

    insert_edge(&connection, "concept-a", "concept-b", 2);
    insert_edge(&connection, "concept-a", "concept-c", 2);
    insert_edge(&connection, "concept-b", "concept-d", 2);
    insert_edge(&connection, "concept-c", "concept-e", 1);
}

fn insert_concept(connection: &rusqlite::Connection, name: &str) {
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
            params![name],
        )
        .expect("insert concept");
}

fn insert_edge(connection: &rusqlite::Connection, a: &str, b: &str, weight: i64) {
    let (concept_a, concept_b) = if a < b { (a, b) } else { (b, a) };
    let source_files = serde_json::to_string(
        &(0..weight)
            .map(|index| (format!("test://f{index}"), 1.0_f64))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )
    .expect("serialize source files");

    connection
        .execute(
            "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
             VALUES (?1, ?2, ?3, ?4)",
            params![concept_a, concept_b, weight, source_files],
        )
        .expect("insert edge");
}

fn visualize(cfg: &Config, payload: serde_json::Value) -> String {
    let context = markdown_context(cfg);
    search::visualize(&context, &payload).expect("visualize")
}

#[test]
fn visualize_with_valid_concept_returns_mermaid_diagram() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithValidConcept_ReturnsMermaidDiagram.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(!result.is_empty(), "result should not be empty");
    assert!(
        result.starts_with("graph TD"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("-->"),
        "expected Mermaid edge syntax: {result}"
    );
    assert!(
        result.contains("concept_b") || result.contains("concept_c"),
        "expected connected concepts in diagram: {result}"
    );
}

#[test]
fn visualize_includes_co_occurrence_counts() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_IncludesCoOccurrenceCounts.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.contains("-->|2|"),
        "expected edge labels in output: {result}"
    );
}

#[test]
fn visualize_respects_depth_parameter() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_RespectsDepthParameter.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let depth_one = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 50
        }),
    );
    let depth_two = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 2,
            "maxNodes": 50
        }),
    );

    let depth_one_lines = depth_one
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let depth_two_lines = depth_two
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    assert!(
        depth_two_lines >= depth_one_lines,
        "depth 2 should include at least as many lines as depth 1:\n{depth_one}\n---\n{depth_two}"
    );
}

#[test]
fn visualize_respects_max_nodes_parameter() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_RespectsMaxNodesParameter.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let max_five = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 3,
            "maxNodes": 5
        }),
    );
    let max_fifty = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 3,
            "maxNodes": 50
        }),
    );

    let lines_five = max_five
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let lines_fifty = max_fifty
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    assert!(
        lines_five <= lines_fifty,
        "lower maxNodes should not produce more lines:\n{max_five}\n---\n{max_fifty}"
    );
}

#[test]
fn visualize_normalizes_concept_names() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_NormalizesConceptNames.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let lower = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );
    let upper = visualize(
        &cfg,
        json!({
            "conceptName": "CONCEPT-A",
            "depth": 1,
            "maxNodes": 10
        }),
    );
    let mixed = visualize(
        &cfg,
        json!({
            "conceptName": "Concept-A",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        lower.starts_with("graph TD"),
        "lowercase should work: {lower}"
    );
    assert!(
        upper.starts_with("graph TD"),
        "uppercase should normalize: {upper}"
    );
    assert!(
        mixed.starts_with("graph TD"),
        "mixed case should normalize: {mixed}"
    );
}

#[test]
fn visualize_with_empty_concept_name_returns_error() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithEmptyConceptName_ReturnsError.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("ERROR"),
        "expected error output: {result}"
    );
    assert!(
        result.contains("required"),
        "missing required detail: {result}"
    );
}

#[test]
fn visualize_with_null_concept_name_returns_error() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithNullConceptName_ReturnsError.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": serde_json::Value::Null,
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("ERROR"),
        "expected error output: {result}"
    );
}

#[test]
fn visualize_with_non_existent_concept_returns_not_found_message() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithNonExistentConcept_ReturnsNotFoundMessage.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "non-existent-concept-xyz",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.contains("not found"),
        "missing not-found detail: {result}"
    );
    assert!(
        result.to_ascii_lowercase().contains("sync"),
        "missing sync guidance: {result}"
    );
}

#[test]
fn visualize_with_invalid_depth_returns_error() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithInvalidDepth_ReturnsError.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    for invalid_depth in [0, -1, 6, 10] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": invalid_depth,
                "maxNodes": 10
            }),
        );

        assert!(
            result.starts_with("ERROR"),
            "depth {invalid_depth} should fail: {result}"
        );
        assert!(
            result.contains("depth") && result.contains("between"),
            "depth range detail missing for {invalid_depth}: {result}"
        );
    }
}

#[test]
fn visualize_with_invalid_max_nodes_returns_error() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithInvalidMaxNodes_ReturnsError.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    for invalid_max_nodes in [0, 4, 101, 1000] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": 1,
                "maxNodes": invalid_max_nodes
            }),
        );

        assert!(
            result.starts_with("ERROR"),
            "maxNodes {invalid_max_nodes} should fail: {result}"
        );
        assert!(
            result.contains("maxNodes") && result.contains("between"),
            "maxNodes range detail missing for {invalid_max_nodes}: {result}"
        );
    }
}

#[test]
fn visualize_with_isolated_concept_returns_empty_graph() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_WithIsolatedConcept_ReturnsEmptyGraph.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "isolated-concept",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("graph TD"),
        "expected Mermaid output: {result}"
    );
    assert!(
        result.contains("No connections found") || result.contains("isolated_concept"),
        "expected isolated-node indicator: {result}"
    );
}

#[test]
fn visualize_generates_valid_mermaid_node_ids() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs Visualize_GeneratesValidMermaidNodeIds.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("graph TD"),
        "unexpected output: {result}"
    );

    let edge_regex = Regex::new(r"\w+ -->\|\d+\| \w+").expect("edge regex");
    assert!(
        edge_regex.is_match(&result),
        "expected Mermaid edge line with node identifiers: {result}"
    );

    for line in result
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
    {
        assert!(
            line.contains("-->") || line.contains('['),
            "expected Mermaid syntax on line: {line}"
        );
    }
}

#[test]
fn random_walk_with_valid_concept_returns_walk_sequence() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs RandomWalk_WithValidConcept_ReturnsWalkSequence.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let walk = search::random_walk(&connection, "concept-a", Some(5)).expect("random walk");

    assert!(!walk.is_empty(), "walk should contain at least one concept");
    assert_eq!(
        walk[0], "concept-a",
        "walk should start at requested concept"
    );
    assert!(
        walk.len() <= 5,
        "walk length should not exceed request: {walk:?}"
    );
}

#[test]
fn random_walk_terminates_at_dead_end() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs RandomWalk_TerminatesAtDeadEnd.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let walk = search::random_walk(&connection, "isolated-concept", Some(10)).expect("random walk");

    assert_eq!(
        walk.len(),
        1,
        "isolated concept walk should stop immediately"
    );
    assert_eq!(walk[0], "isolated-concept");
}

#[test]
fn random_walk_produces_different_results() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs RandomWalk_ProducesDifferentResults.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let unique_walks = (0..10)
        .map(|_| {
            search::random_walk(&connection, "concept-a", Some(5))
                .expect("random walk")
                .join(",")
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert!(
        unique_walks.len() > 1,
        "expected multiple random walks to diverge, got {unique_walks:?}"
    );
}

#[test]
fn random_walk_has_default_walk_length() {
    // T-1.6: RTM NFR-1 — ported from GraphAnalyzerTests.cs RandomWalk_HasDefaultWalkLength.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let walk = search::random_walk(&connection, "concept-a", None).expect("random walk");

    assert!(walk.len() <= 10, "default walk length should be 10 or less");
}
