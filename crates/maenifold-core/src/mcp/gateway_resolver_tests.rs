use super::gateway_catalog::GatewayCatalog;
use super::gateway_resolver::{parse_and_validate_output, render_compact_catalog, resolve_intent};
use super::gateway_resolver_types::ResolverError;
use super::stdio_pool::StdioPool;
use super::tool_routing::ToolRoutingRegistry;
use crate::acp_config::IntentResolverSettings;
use serde_json::json;

async fn catalog() -> GatewayCatalog {
    let pool = StdioPool::start(Vec::new()).await.expect("pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    GatewayCatalog::build(&[], &[], &routing).expect("catalog")
}

fn output(candidates: serde_json::Value) -> String {
    json!({
        "intentSummary": "memory operation",
        "candidates": candidates,
    })
    .to_string()
}

fn candidate(command: &str, confidence: f64, safety: &str, exact: bool) -> serde_json::Value {
    json!({
        "domain": "maenifold.memory",
        "command": command,
        "confidence": confidence,
        "reason": "matched memory wording",
        "safetyClass": safety,
        "requiresExactExecute": exact,
        "parametersDraft": {}
    })
}

#[tokio::test]
async fn gateway_resolver_clear_route_validates() {
    let catalog = catalog().await;
    let parsed = parse_and_validate_output(
        &catalog,
        &output(json!([candidate("ReadMemory", 0.98, "read", false)])),
        None,
    )
    .expect("valid route");
    assert_eq!(parsed.candidates[0].command, "ReadMemory");
}

#[tokio::test]
async fn gateway_resolver_ambiguous_route_is_ranked_and_limited() {
    let catalog = catalog().await;
    let parsed = parse_and_validate_output(
        &catalog,
        &output(json!([
            candidate("SearchMemories", 0.72, "read", false),
            candidate("ReadMemory", 0.91, "read", false)
        ])),
        Some(2),
    )
    .expect("valid ambiguous route");
    assert_eq!(parsed.candidates[0].command, "ReadMemory");
    assert_eq!(parsed.candidates.len(), 2);
}

#[tokio::test]
async fn gateway_resolver_malformed_json_returns_structured_error() {
    let catalog = catalog().await;
    let err = parse_and_validate_output(&catalog, "not json", None).expect_err("malformed");
    assert!(matches!(err, ResolverError::Malformed { .. }));
    assert!(err.fallback().contains("GatewayIndex"));
}

#[tokio::test]
async fn gateway_resolver_disabled_without_invoking_acp() {
    let catalog = catalog().await;
    let settings = IntentResolverSettings::default();
    let err = resolve_intent(
        &catalog,
        &settings,
        "read memory",
        std::path::Path::new("."),
    )
    .await
    .expect_err("disabled");
    assert!(matches!(err, ResolverError::Disabled { .. }));
}

#[tokio::test]
async fn gateway_resolver_timeout_is_structured() {
    let catalog = catalog().await;
    let root = tempfile::tempdir().expect("root");
    std::fs::write(
        root.path().join("maenifold.json"),
        json!({"agents":{"resolver":{"command":"/bin/echo claude"}}}).to_string(),
    )
    .expect("config");
    let settings = IntentResolverSettings {
        enabled: true,
        agent: Some("resolver".to_string()),
        timeout_seconds: Some(0),
        ..IntentResolverSettings::default()
    };
    let err = resolve_intent(&catalog, &settings, "read memory", root.path())
        .await
        .expect_err("timeout");
    assert!(matches!(err, ResolverError::Timeout { .. }));
}

#[tokio::test]
async fn gateway_resolver_safety_mismatch_is_rejected() {
    let catalog = catalog().await;
    let err = parse_and_validate_output(
        &catalog,
        &output(json!([candidate("WriteMemory", 0.95, "read", true)])),
        None,
    )
    .expect_err("safety mismatch");
    assert!(matches!(err, ResolverError::Malformed { .. }));
}

#[tokio::test]
async fn gateway_resolver_catalog_prompt_is_bounded() {
    let catalog = catalog().await;
    let rendered = render_compact_catalog(&catalog, 800);
    assert!(rendered.chars().count() <= 800);
}
