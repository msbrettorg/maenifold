use super::*;
use crate::acp_config::IntentResolverSettings;
use crate::mcp::gateway_registry::GatewayRegistry;
use crate::mcp::gateway_runtime::GatewayState;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::StdioPool;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

async fn state() -> GatewayState {
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
    GatewayState::new(
        pool,
        resources,
        registry,
        IntentResolverSettings::default(),
        PathBuf::from("."),
    )
}

fn item(domain: &str, command: &str, parameters: Value) -> BatchItemParams {
    BatchItemParams {
        domain: domain.to_string(),
        command: command.to_string(),
        parameters: parameters.as_object().expect("object params").clone(),
    }
}

fn params(calls: Vec<BatchItemParams>, continue_on_error: bool) -> GatewayBatchParams {
    GatewayBatchParams {
        operation: "submit".to_string(),
        calls: Some(calls),
        continue_on_error: Some(continue_on_error),
        batch_id: None,
    }
}

fn text(result: &CallToolResult) -> String {
    result.content[0]
        .raw
        .as_text()
        .expect("text content")
        .text
        .clone()
}

#[tokio::test]
async fn gateway_batch_rejects_more_than_max_items_before_validation() {
    let state = state().await;
    let calls = (0..=MAX_BATCH_ITEMS)
        .map(|_| item("missing-domain", "missing-command", json!({})))
        .collect();

    let err = handle_gateway_batch(&state, &BatchState::new(), params(calls, true))
        .await
        .expect_err("oversized batch should fail");

    let message = err.to_string();
    assert!(message.contains("at most 100"), "{message}");
    assert!(!message.contains("Unknown gateway domain"), "{message}");
}

#[tokio::test]
async fn gateway_batch_rejects_invalid_native_params_before_execution() {
    let state = state().await;
    let invalid = item("maenifold.graph", "BuildContext", json!({}));

    let err = handle_gateway_batch(&state, &BatchState::new(), params(vec![invalid], true))
        .await
        .expect_err("native schema validation should reject missing conceptName");

    let message = err.to_string();
    assert!(
        message.contains("Missing required parameter(s)"),
        "{message}"
    );
    assert!(message.contains("BuildContext"), "{message}");
    assert!(message.contains("conceptName"), "{message}");
}

#[test]
fn batch_state_eviction_removes_oldest_after_cap() {
    let state = BatchState::new();
    for index in 0..=1000 {
        let batch_id = format!("batch-{index}");
        state.insert(BatchResult {
            batch_id,
            status: "completed".to_string(),
            items: vec![],
        });
    }

    assert!(state.get("batch-0").is_none());
    assert!(state.get("batch-1").is_some());
    assert!(state.get("batch-1000").is_some());
}

#[tokio::test]
async fn gateway_batch_continue_on_error_false_skips_remaining_items() {
    let state = state().await;
    let first = item(
        "maenifold.memory",
        "MoveMemory",
        json!({ "source": "__missing__", "destination": "unused.md" }),
    );
    let second = item(
        "maenifold.memory",
        "ReadMemory",
        json!({ "identifier": "__also_missing__" }),
    );

    let result = handle_gateway_batch(
        &state,
        &BatchState::new(),
        params(vec![first, second], false),
    )
    .await
    .expect("batch should complete with failed item result");
    let rendered: Value = serde_json::from_str(&text(&result)).expect("batch json");

    assert_eq!(rendered["status"], "failed");
    assert_eq!(rendered["items"][0]["status"], "failed");
    assert_eq!(rendered["items"][1]["status"], "skipped");
}

#[test]
fn gateway_batch_status_returns_known_and_batch_not_found_for_unknown() {
    let batch_state = BatchState::new();
    batch_state.insert(BatchResult {
        batch_id: "known".to_string(),
        status: "completed".to_string(),
        items: vec![],
    });

    let known = batch_status(
        &batch_state,
        GatewayBatchParams {
            operation: "status".to_string(),
            calls: None,
            continue_on_error: None,
            batch_id: Some("known".to_string()),
        },
    )
    .expect("known batch");
    assert!(text(&known).contains("\"batchId\": \"known\""));

    let unknown = batch_status(
        &batch_state,
        GatewayBatchParams {
            operation: "status".to_string(),
            calls: None,
            continue_on_error: None,
            batch_id: Some("missing".to_string()),
        },
    )
    .expect_err("unknown batch");
    assert!(format!("{unknown:?}").contains("batchNotFound"));
}

#[tokio::test]
async fn gateway_batch_arguments_reject_legacy_items_field() {
    let state = state().await;
    let err = handle_gateway_batch_arguments(
        &state,
        &BatchState::new(),
        json!({
            "operation": "submit",
            "continueOnError": true,
            "items": []
        }),
    )
    .await
    .expect_err("legacy items field should be rejected");

    let message = err.to_string();
    assert!(message.contains("unknown field `items`"), "{message}");
}

#[tokio::test]
async fn gateway_batch_arguments_accept_empty_calls_and_render_items_response() {
    let state = state().await;
    let result = handle_gateway_batch_arguments(
        &state,
        &BatchState::new(),
        json!({
            "operation": "submit",
            "continueOnError": true,
            "calls": []
        }),
    )
    .await
    .expect("empty calls should succeed");
    let rendered: Value = serde_json::from_str(&text(&result)).expect("batch json");

    assert_eq!(rendered["status"], "completed");
    assert_eq!(rendered["items"], json!([]));
}
