use crate::mcp::sampling_session_registry::{
    SamplingDecisionRecord, SamplingDispatch, SamplingRegistryError, SamplingSessionRegistry,
    SamplingSessionStatus, TokenUsage, is_terminal_status,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicBool};
use uuid::Uuid;

#[tokio::test]
async fn new_registry_is_empty() {
    assert_eq!(SamplingSessionRegistry::new().snapshot().await.len(), 0);
}

#[tokio::test]
async fn insert_then_snapshot_returns_one_entry() {
    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;

    registry.insert(dispatch).await;

    let snapshot = registry.snapshot().await;
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].sampling_request_id, id);
}

#[tokio::test]
async fn update_status_transitions_pending_to_streaming() {
    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    registry
        .update_status(id, SamplingSessionStatus::Streaming)
        .await
        .expect("known dispatch should update");

    let snapshot = registry.snapshot().await;
    assert_eq!(snapshot[0].status, SamplingSessionStatus::Streaming);
}

#[tokio::test]
async fn test_update_acp_session_transitions_status() {
    let registry = SamplingSessionRegistry::new();
    let mut dispatch = dispatch_fixture("test-server");
    dispatch.status = SamplingSessionStatus::Dispatching;
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    registry
        .update_acp_session(id, "abc".to_string())
        .await
        .expect("known dispatch should store ACP session id");
    registry
        .update_status(id, SamplingSessionStatus::Streaming)
        .await
        .expect("dispatching session should become streaming");
    registry
        .update_status(id, SamplingSessionStatus::Completed)
        .await
        .expect("streaming session should become completed");

    let snapshot = registry.snapshot().await;
    assert_eq!(snapshot[0].acp_session_id.as_deref(), Some("abc"));
    assert_eq!(snapshot[0].status, SamplingSessionStatus::Completed);
}

#[tokio::test]
async fn test_update_acp_session_is_referenced_from_dispatch_path() {
    let source = include_str!("sampling_client_flow.rs");
    assert!(source.contains(".update_acp_session(dispatch_id, acp_session_id)"));
    assert!(source.contains("session_id_rx.await"));

    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    registry
        .update_acp_session(id, "live-acp-session".to_string())
        .await
        .expect("test call keeps symbol reachable from crate tests");
}

#[tokio::test]
async fn update_status_unknown_id_returns_not_found() {
    let registry = SamplingSessionRegistry::new();

    assert_eq!(
        registry
            .update_status(Uuid::new_v4(), SamplingSessionStatus::Completed)
            .await,
        Err(SamplingRegistryError::NotFound)
    );
}

#[tokio::test]
async fn record_usage_persists_token_counts() {
    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    registry
        .record_usage(
            id,
            TokenUsage {
                input_tokens: 42,
                output_tokens: 17,
            },
        )
        .await
        .expect("known dispatch should record usage");

    let snapshot = registry.snapshot().await;
    assert_eq!(
        snapshot[0].token_usage,
        Some(TokenUsage {
            input_tokens: 42,
            output_tokens: 17,
        })
    );
}

#[tokio::test]
async fn reset_tool_calls_zeroes_count_without_touching_turn_count() {
    let registry = SamplingSessionRegistry::new();
    let mut dispatch = dispatch_fixture("test-server");
    dispatch.tool_call_count = 5;
    dispatch.turn_count = 3;
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    registry
        .reset_tool_calls(id)
        .await
        .expect("known dispatch should reset tool calls");

    let stored = registry
        .get(id)
        .await
        .expect("dispatch should remain stored");
    assert_eq!(stored.tool_call_count, 0);
    assert_eq!(stored.turn_count, 3);
}

#[test]
fn tool_loop_budget_for_returns_per_server_override_or_default() {
    let registry = SamplingSessionRegistry::with_tool_loop_budgets(
        10,
        HashMap::from([("foo".to_string(), 25)]),
    );

    assert_eq!(registry.tool_loop_budget_for("foo"), 25);
    assert_eq!(registry.tool_loop_budget_for("bar"), 10);
}

#[tokio::test]
async fn budget_exceeded_status_is_non_terminal_until_cancelled() {
    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;
    let status_handle = Arc::clone(&dispatch.status_handle);
    registry.insert(dispatch).await;

    registry
        .update_status(
            id,
            SamplingSessionStatus::BudgetExceeded {
                allowed_so_far: 10,
                budget: 10,
            },
        )
        .await
        .expect("budget exceeded should be a resumable status");

    assert!(!is_terminal_status(
        &SamplingSessionStatus::BudgetExceeded {
            allowed_so_far: 10,
            budget: 10,
        }
    ));
    assert!(!status_handle.load(Ordering::SeqCst));

    registry
        .update_status(id, SamplingSessionStatus::Cancelled)
        .await
        .expect("cancel should terminate the session");

    assert!(is_terminal_status(&SamplingSessionStatus::Cancelled));
    assert!(status_handle.load(Ordering::SeqCst));
}

#[tokio::test]
async fn increment_tool_calls_accumulates_sequential_and_joined_increments() {
    let registry = SamplingSessionRegistry::new();
    let dispatch = dispatch_fixture("test-server");
    let id = dispatch.sampling_request_id;
    registry.insert(dispatch).await;

    assert_eq!(
        registry
            .increment_tool_calls(id, 3)
            .await
            .expect("first increment should succeed"),
        3
    );
    assert_eq!(
        registry
            .increment_tool_calls(id, 2)
            .await
            .expect("second increment should succeed"),
        5
    );

    let (first, second) = tokio::join!(
        registry.increment_tool_calls(id, 4),
        registry.increment_tool_calls(id, 6),
    );

    first.expect("joined increment should succeed");
    second.expect("joined increment should succeed");
    let stored = registry
        .get(id)
        .await
        .expect("dispatch should remain stored");
    assert_eq!(stored.tool_call_count, 15);
}

fn dispatch_fixture(server_name: &str) -> SamplingDispatch {
    SamplingDispatch {
        sampling_request_id: Uuid::from_u128(0x11111111111111111111111111111111),
        app_session_id: Uuid::from_u128(0x22222222222222222222222222222222),
        acp_session_id: None,
        server_name: server_name.to_string(),
        model: "copilot".to_string(),
        runtime_kind: "copilot".to_string(),
        role_name: "copilot".to_string(),
        status: SamplingSessionStatus::PendingApproval,
        decision: SamplingDecisionRecord::AllowOnce,
        created_at: "2026-05-13T00:00:00Z".to_string(),
        token_usage: None,
        tool_call_count: 0,
        tool_loop_budget: 10,
        turn_count: 0,
        status_handle: Arc::new(AtomicBool::new(false)),
    }
}
