use super::domain_mapping::SafetyClass;
use super::gateway_catalog::GatewayCatalog;
use super::gateway_experiment::{
    ExperimentReport, LEVELS, LevelReport, score_fixture, write_report,
};
use super::gateway_experiment_fixtures::{FIXTURES, FixtureCategory};
use super::gateway_resolver_types::{ResolverCandidate, ResolverOutput};
use super::stdio_pool::StdioPool;
use super::tool_routing::ToolRoutingRegistry;
use crate::acp_config::IntentResolverSettings;
use serde_json::Map;

#[test]
fn fixture_set_has_required_shape() {
    assert_eq!(FIXTURES.len(), 32);
    for (category, expected) in [
        (FixtureCategory::ClearRead, 8),
        (FixtureCategory::ClearMutate, 6),
        (FixtureCategory::Ambiguous, 6),
        (FixtureCategory::Broad, 4),
        (FixtureCategory::MalformedParams, 3),
        (FixtureCategory::Adversarial, 5),
    ] {
        assert_eq!(
            FIXTURES.iter().filter(|f| f.category == category).count(),
            expected
        );
    }
}

#[test]
fn scoring_separates_route_accuracy_from_mutation_safety() {
    let fixture = FIXTURES
        .iter()
        .find(|f| f.category == FixtureCategory::Adversarial)
        .unwrap();
    let output = ResolverOutput {
        intent_summary: "unsafe".to_string(),
        candidates: vec![candidate(
            "maenifold.memory",
            "WriteMemory",
            SafetyClass::Write,
            0.99,
        )],
    };
    let scored = score_fixture(fixture, "medium", Ok(output), 7);
    assert!(!scored.route_correct);
    assert!(!scored.safety_correct);
    assert_eq!(scored.mutating_candidates, 1);
}

#[test]
fn report_structure_serializes_recommendation_and_metrics() {
    let report = ExperimentReport {
        model: "gpt-5.4-mini".to_string(),
        fixture_count: FIXTURES.len(),
        recommended_level: Some("medium".to_string()),
        levels: vec![LevelReport {
            level: "medium".to_string(),
            total: 1,
            route_accuracy: 1.0,
            mutation_safety_score: 1.0,
            malformed_output_rate: 0.0,
            average_confidence: 0.95,
            average_latency_ms: 12.0,
            acp_error_count: 0,
            adversarial_failures: 0,
            results: Vec::new(),
        }],
    };
    let value = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(value["model"], "gpt-5.4-mini");
    assert_eq!(value["recommendedLevel"], "medium");
    assert!(value["levels"][0].get("mutationSafetyScore").is_some());
}

#[test]
fn report_writer_uses_sprint_output_directory() {
    let root = tempfile::tempdir().expect("root");
    let report = ExperimentReport {
        model: "gpt-5.4-mini".to_string(),
        fixture_count: 0,
        recommended_level: None,
        levels: Vec::new(),
    };
    let path = write_report(root.path(), &report).expect("report writes");
    assert!(
        path.to_string_lossy()
            .contains("outputs/intent-resolver/sprint-221/experiment-")
    );
    assert!(path.exists());
}

#[tokio::test]
async fn experiment_catalog_uses_native_gate_without_standalone_server() {
    let catalog = experiment_catalog().await;
    let domains = catalog.domains();
    assert!(domains.iter().any(|domain| domain.name == "maenifold.gate"));
    assert!(!domains.iter().any(|domain| domain.name == "gate"));
    assert!(catalog.commands("maenifold.gate").is_ok());
    assert!(catalog.commands("gate").is_err());
}

#[tokio::test]
#[ignore = "requires configured direct inference backend; run with --ignored --nocapture"]
async fn live_reasoning_level_experiment_writes_report() {
    let root = std::env::var("MAENIFOLD_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let catalog = experiment_catalog().await;
    let settings = IntentResolverSettings {
        enabled: true,
        max_candidates: Some(5),
        timeout_seconds: Some(60),
        ..IntentResolverSettings::default()
    };
    let report =
        super::gateway_experiment::run_experiment(&catalog, &settings, &root, LEVELS).await;
    let path = write_report(&root, &report).expect("report writes");
    println!("{}", path.display());
    assert_eq!(report.fixture_count, FIXTURES.len());
}

fn candidate(
    domain: &str,
    command: &str,
    safety_class: SafetyClass,
    confidence: f64,
) -> ResolverCandidate {
    ResolverCandidate {
        domain: domain.to_string(),
        command: command.to_string(),
        confidence,
        reason: "test".to_string(),
        safety_class,
        requires_exact_execute: safety_class != SafetyClass::Read,
        parameters_draft: Map::new(),
    }
}

async fn experiment_catalog() -> GatewayCatalog {
    let pool = StdioPool::start(
        Vec::new(),
        std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    GatewayCatalog::build(&[], &[], &routing).expect("catalog builds")
}
