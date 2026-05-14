use super::*;
use rmcp::model::{ElicitationSchema, PrimitiveSchema, StringSchema};

fn params() -> CreateElicitationRequestParams {
    CreateElicitationRequestParams::FormElicitationParams {
        meta: None,
        message: "Need input".to_string(),
        requested_schema: ElicitationSchema::builder()
            .mark_required("reason")
            .property("reason", PrimitiveSchema::String(StringSchema::new()))
            .build()
            .expect("schema should be valid"),
    }
}

#[tokio::test]
async fn pending_events_include_registered_request() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let _receiver = registry.register_elicitation(params()).await;
    assert_eq!(registry.pending_events().await.len(), 1);
}

#[tokio::test]
async fn subscribe_before_snapshot_does_not_lose_concurrent_register() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let mut live_events = registry.subscribe();
    let register_registry = Arc::clone(&registry);
    let register_task = tokio::spawn(async move {
        let _receiver = register_registry.register_elicitation(params()).await;
    });
    let snapshot = registry.pending_events().await;
    register_task
        .await
        .expect("concurrent register task should finish");
    let live_event = tokio::time::timeout(Duration::from_secs(1), live_events.recv())
        .await
        .expect("live event should arrive")
        .expect("live event channel should be open");
    let snapshot_has_live_event = snapshot.iter().any(|event| event.id == live_event.id);
    assert!(
        snapshot_has_live_event
            || registry
                .pending_events()
                .await
                .iter()
                .any(|event| event.id == live_event.id),
        "subscribing before the snapshot must retain either snapshot or live delivery"
    );
}

#[tokio::test]
async fn duplicate_response_reports_gone() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let receiver = registry.register_elicitation(params()).await;
    let id = registry.pending_events().await[0].id;
    let response = ElicitationResponse {
        action: UserPromptAction::Cancel,
        content: None,
    };
    registry
        .respond_typed(UserPromptResponse::Elicitation {
            id,
            response: response.clone(),
        })
        .await
        .expect("first response should resolve");
    assert_eq!(receiver.await.expect("response delivered"), response);
    assert_eq!(
        registry
            .respond_typed(UserPromptResponse::Elicitation { id, response })
            .await,
        Err(RespondError::Gone)
    );
}
