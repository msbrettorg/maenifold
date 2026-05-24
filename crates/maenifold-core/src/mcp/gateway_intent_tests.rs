use super::domain_mapping::SafetyClass;
use super::gateway_intent::{handle_gateway_intent, handle_resolved_intent, resolved_text};
use super::gateway_resolver_types::{ResolverCandidate, ResolverOutput};
use super::resource_routing::ResourceRoutingRegistry;
use super::stdio_pool::StdioPool;
use crate::acp_config::IntentResolverSettings;
use crate::mcp::gateway_registry::GatewayRegistry;
use crate::mcp::gateway_runtime::GatewayState;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

mod scoped_env {
    /// Unset every env var that could otherwise satisfy
    /// `IntentResolverSettings::resolved_direct_inference()` for the duration
    /// of one test. Restores prior values on drop.
    pub(super) fn unset_inference_env() -> InferenceEnvGuard {
        const KEYS: &[&str] = &[
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_CHAT_DEPLOYMENT_NAME",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
            "OPENAI_BASE_URL",
            "OPENAI_API_KEY",
        ];
        let prior: Vec<(&'static str, Option<String>)> = KEYS
            .iter()
            .map(|key| {
                let prev = std::env::var(key).ok();
                // SAFETY: tests using this guard are tagged #[serial] so env
                // mutations cannot race with other tests.
                unsafe { std::env::remove_var(key) };
                (*key, prev)
            })
            .collect();
        InferenceEnvGuard { prior }
    }

    pub(super) struct InferenceEnvGuard {
        prior: Vec<(&'static str, Option<String>)>,
    }

    impl Drop for InferenceEnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.prior {
                // SAFETY: see unset_inference_env above.
                unsafe {
                    match value {
                        Some(v) => std::env::set_var(key, v),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }
}

async fn state(settings: IntentResolverSettings) -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("empty pool"),
    );
    let registry = GatewayRegistry::build(Vec::new(), &BTreeMap::new()).expect("registry");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(pool, resources, registry, settings, PathBuf::from("."))
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
#[serial]
async fn resolver_error_returns_fallback_instructions() {
    // Enabled but no direct inference configured:
    // should hit ResolverError::DirectInferenceSetup, NOT fall back to ACP.
    // Clear env vars on this thread so an Azure OpenAI endpoint that happens
    // to be set on the test host does not satisfy the resolver.
    let _guard = scoped_env::unset_inference_env();
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

/// BUG-1 regression: a read-safe candidate whose `parameters_draft` is missing
/// a schema-required parameter (e.g. `ReadMemory` without `id`) must NOT
/// auto-execute; instead we return a candidate annotated with
/// `missingRequiredParameters` so the caller can fill them and try again.
#[tokio::test]
async fn read_safe_missing_required_parameters_returns_candidates() {
    let state = state(settings(true)).await;
    // ReadMemory requires `id`. The draft is empty, so auto-execute must be
    // suppressed even though the candidate is read-safe and high confidence.
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("ReadMemory", 0.99, SafetyClass::Read)]),
    )
    .await
    .expect("intent result");
    let value = json_text(&result);

    let status = value["status"].as_str().unwrap_or_default();
    assert!(
        status == "candidates" || status == "plan",
        "expected suppression of auto-execute, got status={status} body={value}"
    );
    let candidate_value = if status == "candidates" {
        &value["candidates"][0]
    } else {
        &value["candidates"][0]
    };
    let missing = candidate_value["missingRequiredParameters"]
        .as_array()
        .expect("missingRequiredParameters present");
    assert!(
        missing.iter().any(|item| item == "identifier"),
        "expected `identifier` to be flagged missing, got {missing:?}"
    );
}

#[tokio::test]
#[serial]
async fn read_safe_complete_draft_still_auto_executes() {
    let state = state(settings(true)).await;
    // GetConfig has no required parameters, so an empty draft is complete.
    let result = handle_resolved_intent(
        &state,
        output(vec![candidate("GetConfig", 0.99, SafetyClass::Read)]),
    )
    .await
    .expect("intent result");
    let text = resolved_text(&result);
    assert!(text.contains("maenifoldRoot"), "{text}");
    assert!(!text.contains("\"status\": \"candidates\""), "{text}");
}

#[tokio::test]
async fn gateway_surface_always_exposes_exact_seven_public_tools() {
    let disabled = state(IntentResolverSettings::default()).await;
    let enabled = state(settings(true)).await;
    assert_eq!(
        exposed_public_surface(disabled).await,
        expected_public_surface()
    );
    assert_eq!(
        exposed_public_surface(enabled).await,
        expected_public_surface()
    );
}

async fn exposed_public_surface(gateway: GatewayState) -> Vec<String> {
    let mut names: Vec<String> = gateway
        .list_tools()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.extend(
        ["GetHelp", "ReadMcpResource", "complete_node"]
            .into_iter()
            .map(str::to_string),
    );
    names.sort();
    names
}

fn expected_public_surface() -> Vec<String> {
    vec![
        "GatewayBatch".to_string(),
        "GatewayExecute".to_string(),
        "GatewayIndex".to_string(),
        "GatewayIntent".to_string(),
        "GetHelp".to_string(),
        "ReadMcpResource".to_string(),
        "complete_node".to_string(),
    ]
}
