use crate::mcp::sampling_session_registry::{
    SamplingDispatch, SamplingRegistryError, SamplingSessionRegistry, SamplingSessionStatus,
    TokenUsage,
};
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

fn dispatch_fixture(server_name: &str) -> SamplingDispatch {
    SamplingDispatch {
        sampling_request_id: Uuid::from_u128(0x11111111111111111111111111111111),
        app_session_id: Uuid::from_u128(0x22222222222222222222222222222222),
        acp_session_id: None,
        server_name: server_name.to_string(),
        model: "copilot".to_string(),
        status: SamplingSessionStatus::PendingApproval,
        created_at: "2026-05-13T00:00:00Z".to_string(),
        token_usage: None,
    }
}
