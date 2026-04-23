// T-01.16: visualize tool tests extracted verbatim from graph/analyzer.rs.

use super::visualize;
use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn visualize_test_context(output_format: OutputFormat) -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage/tools");
    fs::write(
        config.assets_path.join("usage/tools/visualize.md"),
        "# Visualize help",
    )
    .expect("write visualize help");
    (
        tmp,
        ToolContext::new(config, output_format),
        ConfigResetGuard,
    )
}

fn seed_graph(context: &ToolContext, concepts: &[&str], edges: &[(&str, &str, i64)]) {
    GraphDatabase::initialize_database(&context.config).expect("initialize db");
    let connection = db::open_connection(&context.config.database_path).expect("open db");

    for concept in concepts {
        connection
            .execute(
                "INSERT OR IGNORE INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, '2026-01-01T00:00:00Z', 1)",
                [*concept],
            )
            .expect("insert concept");
    }

    for (source, target, weight) in edges {
        connection
            .execute(
                "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, '')",
                rusqlite::params![source, target, weight],
            )
            .expect("insert edge");
    }
}

#[test]
#[serial]
fn visualize_mermaid_output_stays_stable() {
    let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Markdown);
    seed_graph(
        &context,
        &["machine-learning", "neural-networks", "deep-learning"],
        &[
            ("machine-learning", "neural-networks", 12),
            ("machine-learning", "deep-learning", 8),
        ],
    );

    let result = visualize(
        &context,
        &json!({
            "conceptName": "machine-learning",
            "depth": 1,
            "maxNodes": 30
        }),
    )
    .expect("visualize should succeed");

    assert_eq!(
        result,
        "graph TD\n    machine_learning -->|12| neural_networks\n    machine_learning -->|8| deep_learning\n"
    );
}

#[test]
#[serial]
fn visualize_graph_format_returns_structured_payload() {
    let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Json);
    seed_graph(
        &context,
        &["machine-learning", "neural-networks", "deep-learning"],
        &[
            ("machine-learning", "neural-networks", 12),
            ("machine-learning", "deep-learning", 8),
        ],
    );

    let result = visualize(
        &context,
        &json!({
            "conceptName": "machine-learning",
            "depth": 1,
            "maxNodes": 30,
            "format": "graph"
        }),
    )
    .expect("visualize should succeed");

    let payload: Value = serde_json::from_str(&result).expect("json payload");
    let data = payload.get("data").expect("data field");
    assert_eq!(data.get("center"), Some(&json!("machine-learning")));

    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert!(nodes.iter().any(|node| {
        node.get("id") == Some(&json!("machine-learning"))
            && node.get("label") == Some(&json!("machine-learning"))
            && node.get("depth") == Some(&json!(0))
    }));
    assert!(nodes.iter().any(|node| {
        node.get("id") == Some(&json!("neural-networks"))
            && node.get("label") == Some(&json!("neural-networks"))
            && node.get("depth") == Some(&json!(1))
    }));

    let edges = data
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges array");
    assert!(edges.iter().any(|edge| {
        edge.get("source") == Some(&json!("machine-learning"))
            && edge.get("target") == Some(&json!("neural-networks"))
            && edge.get("weight") == Some(&json!(12.0))
    }));
}

#[test]
#[serial]
fn visualize_graph_format_includes_center_when_no_neighbors() {
    let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Json);
    seed_graph(&context, &["solo-concept"], &[]);

    let result = visualize(
        &context,
        &json!({
            "conceptName": "solo-concept",
            "depth": 2,
            "maxNodes": 30,
            "format": "graph"
        }),
    )
    .expect("visualize should succeed");

    let payload: Value = serde_json::from_str(&result).expect("json payload");
    let data = payload.get("data").expect("data field");
    assert_eq!(data.get("center"), Some(&json!("solo-concept")));
    assert_eq!(
        data.get("nodes"),
        Some(&json!([{
            "id": "solo-concept",
            "label": "solo-concept",
            "depth": 0
        }]))
    );
    assert_eq!(data.get("edges"), Some(&json!([])));
}
