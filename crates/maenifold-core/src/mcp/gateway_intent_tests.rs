use super::domain_mapping::SafetyClass;
use super::gateway_catalog::GatewayCatalog;
use super::gateway_handler::GatewayState;
use super::gateway_intent::{handle_gateway_intent, handle_resolved_intent, resolved_text};
use super::gateway_resolver_types::{ResolverCandidate, ResolverOutput};
use super::resource_routing::ResourceRoutingRegistry;
use super::stdio_pool::StdioPool;
use super::tool_routing::ToolRoutingRegistry;
use crate::acp_config::IntentResolverSettings;
use serde_json::{Value, json};
use serial_test::serial;
use std::path::PathBuf;
use std::sync::Arc;

async fn state(settings: IntentResolverSettings) -> GatewayState {
    let pool = Arc::new(StdioPool::start(vec![]).await.expect("empty pool"));
    let routing = ToolRoutingRegistry::build(&[], pool.as_ref()).expect("routing");
    let catalog = GatewayCatalog::build(&[], &[], &routing).expect("catalog");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(
        pool,
        resources,
        catalog,
        vec![],
        settings,
        PathBuf::from("."),
    )
}

fn settings(auto: bool) -> IntentResolverSettings {
    IntentResolverSettings {
        enabled: true,
        auto_execute_read_safe: auto,
        auto_execute_confidence_threshold: 0.9,
        ..IntentResolverSettings::default()
    }
}

fn output(candidates: Vec<ResolverCandidate>) -> ResolverOutput {
    ResolverOutput {
        intent_summary: "resolved test intent".to_string(),
        candidates,
    }
}

fn candidate(command: &str, confidence: f64, safety_class: SafetyClass) -> ResolverCandidate {
    ResolverCandidate {
        domain: match command {
            "GetConfig" => "maenifold.system",
            _ => "maenifold.memory",
        }
        .to_string(),
        command: command.to_string(),
        confidence,
        reason: "matched deterministic catalog".to_string(),
        safety_class,
        requires_exact_execute: safety_class != SafetyClass::Read,
        parameters_draft: serde_json::Map::new(),
    }
}

fn json_text(result: &rmcp::model::CallToolResult) -> Value {
    serde_json::from_str(&resolved_text(result)).expect("json result")
}

#[tokio::test]
#[serial]
async fn read_safe_single_high_confidence_auto_executes() {
    let state = state(settings(true)).await;
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("GetConfig", 0.96, SafetyClass::Read)]),
    )
    .await
    .expect("intent result");
    let text = resolved_text(&result);

    assert!(text.contains("maenifoldRoot"), "{text}");
    assert!(!text.contains("\"status\": \"candidates\""), "{text}");
}

#[tokio::test]
async fn read_safe_below_threshold_returns_candidates() {
    let state = state(settings(true)).await;
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("GetConfig", 0.89, SafetyClass::Read)]),
    )
    .await
    .expect("intent result");
    let value = json_text(&result);

    assert_eq!(value["status"], "candidates");
    assert_eq!(value["candidates"][0]["command"], "GetConfig");
}

#[tokio::test]
async fn mutating_candidate_returns_execute_plan() {
    let state = state(settings(true)).await;
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("WriteMemory", 0.99, SafetyClass::Write)]),
    )
    .await
    .expect("intent result");
    let value = json_text(&result);

    assert_eq!(value["status"], "plan");
    assert_eq!(value["plan"]["tool"], "GatewayExecute");
    assert_eq!(value["plan"]["command"], "WriteMemory");
}

#[tokio::test]
async fn auto_execute_disabled_returns_candidates_for_read() {
    let state = state(settings(false)).await;
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("GetConfig", 0.99, SafetyClass::Read)]),
    )
    .await
    .expect("intent result");
    let value = json_text(&result);

    assert_eq!(value["status"], "candidates");
}

#[tokio::test]
async fn resolver_error_returns_fallback_instructions() {
    // Enabled but no direct inference configured AND no client sampling:
    // should hit ResolverError::DirectInferenceSetup, NOT fall back to ACP.
    let mut intent_settings = IntentResolverSettings::default();
    intent_settings.enabled = true;
    let state = state(intent_settings).await;
    let result = handle_gateway_intent(&state, json!({"intent":"read config"}), None)
        .await
        .expect("fallback result");
    let value = json_text(&result);

    assert_eq!(value["status"], "directInferenceSetup");
    assert!(
        value["instructions"]
            .as_str()
            .unwrap()
            .contains("GatewayIndex")
    );
}

#[tokio::test]
async fn ambiguous_candidates_do_not_auto_execute() {
    let state = state(settings(true)).await;
    let result = handle_resolved_intent(
        &state,
        output(vec![
            candidate("GetConfig", 0.99, SafetyClass::Read),
            candidate("ReadMemory", 0.95, SafetyClass::Read),
        ]),
    )
    .await
    .expect("intent result");
    let value = json_text(&result);

    assert_eq!(value["status"], "candidates");
    assert_eq!(value["candidates"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn gateway_intent_visible_when_enabled_without_agent() {
    let disabled = state(IntentResolverSettings::default()).await;
    let enabled = state(settings(true)).await;
    assert!(!disabled.list_tools().iter().any(|tool| tool.name == "GatewayIntent"));
    assert!(enabled.list_tools().iter().any(|tool| tool.name == "GatewayIntent"));
}
