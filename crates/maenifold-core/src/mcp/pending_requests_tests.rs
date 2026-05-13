use crate::mcp::pending_requests::{
    ELICITATION_TIMEOUT, ElicitationResponse, PendingPayload, PendingRequestRegistry, RespondError,
    SAMPLING_BACKSTOP_TIMEOUT, SamplingAction, SamplingDecision, UserPromptAction,
    UserPromptResponse,
};
use rmcp::model::{
    CreateElicitationRequestParams, CreateMessageRequestParams, ElicitationSchema, PrimitiveSchema,
    SamplingMessage, StringSchema,
};
use std::sync::Arc;
use std::time::Duration;

fn elicitation_params() -> CreateElicitationRequestParams {
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

fn sampling_params() -> CreateMessageRequestParams {
    CreateMessageRequestParams::new(vec![SamplingMessage::user_text("Need sampling")], 100)
}

fn cancel_response() -> ElicitationResponse {
    ElicitationResponse {
        action: UserPromptAction::Cancel,
        content: None,
    }
}

fn sampling_decision(action: SamplingAction) -> SamplingDecision {
    SamplingDecision {
        action,
        selected_model: None,
        selected_agent: None,
    }
}

fn require_pending_payload_contract() {
    let _ = std::mem::size_of::<PendingPayload>();
}

#[tokio::test]
async fn register_elicitation_then_respond_succeeds() {
    require_pending_payload_contract();
    let registry = Arc::new(PendingRequestRegistry::new());
    let receiver = registry.register_elicitation(elicitation_params()).await;
    let id = registry.pending_events().await[0].id;
    let response = cancel_response();

    registry
        .respond_typed(UserPromptResponse::Elicitation {
            id,
            response: response.clone(),
        })
        .await
        .expect("elicitation response should be accepted");

    assert_eq!(receiver.await.expect("response delivered"), response);
}

#[tokio::test]
async fn register_sampling_then_respond_succeeds() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let receiver = registry
        .register_sampling(sampling_params(), "test-server-x")
        .await;
    let id = registry.pending_events().await[0].id;
    let response = sampling_decision(SamplingAction::AllowOnce);

    registry
        .respond_typed(UserPromptResponse::Sampling {
            id,
            response: response.clone(),
        })
        .await
        .expect("sampling response should be accepted");

    assert_eq!(receiver.await.expect("decision delivered"), response);
}

#[tokio::test]
async fn cross_kind_rejection_sampling_to_elicitation_fails() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let receiver = registry.register_elicitation(elicitation_params()).await;
    let id = registry.pending_events().await[0].id;

    let result = registry
        .respond_typed(UserPromptResponse::Sampling {
            id,
            response: sampling_decision(SamplingAction::Deny),
        })
        .await;

    assert_eq!(result, Err(RespondError::KindMismatch));
    let response = cancel_response();
    registry
        .respond_typed(UserPromptResponse::Elicitation {
            id,
            response: response.clone(),
        })
        .await
        .expect("correct response should still resolve after mismatch");
    assert_eq!(receiver.await.expect("response delivered"), response);
}

#[tokio::test]
async fn cross_kind_rejection_elicitation_to_sampling_fails() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let receiver = registry
        .register_sampling(sampling_params(), "test-server-x")
        .await;
    let id = registry.pending_events().await[0].id;

    let result = registry
        .respond_typed(UserPromptResponse::Elicitation {
            id,
            response: cancel_response(),
        })
        .await;

    assert_eq!(result, Err(RespondError::KindMismatch));
    let response = sampling_decision(SamplingAction::AllowOnce);
    registry
        .respond_typed(UserPromptResponse::Sampling {
            id,
            response: response.clone(),
        })
        .await
        .expect("correct sampling decision should still resolve after mismatch");
    assert_eq!(receiver.await.expect("decision delivered"), response);
}

#[tokio::test]
async fn sampling_register_does_not_start_elicitation_timeout() {
    tokio::time::pause();
    let registry = Arc::new(PendingRequestRegistry::new());
    let _subscription = registry.subscribe();
    let _receiver = registry
        .register_sampling(sampling_params(), "test-server-x")
        .await;

    tokio::time::advance(ELICITATION_TIMEOUT + Duration::from_secs(1)).await;
    tokio::task::yield_now().await;

    let events = registry.pending_events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "sampling");
}

#[test]
fn sampling_backstop_timeout_constant_is_300s() {
    assert_eq!(SAMPLING_BACKSTOP_TIMEOUT, Duration::from_secs(300));
}

#[tokio::test]
async fn sampling_backstop_expires_pending_after_constant() {
    tokio::time::pause();
    let registry = Arc::new(PendingRequestRegistry::new());
    let _subscription = registry.subscribe();
    let receiver = registry
        .register_sampling(sampling_params(), "test-server-x")
        .await;

    tokio::time::advance(SAMPLING_BACKSTOP_TIMEOUT + Duration::from_secs(1)).await;
    tokio::task::yield_now().await;

    assert!(registry.pending_events().await.is_empty());
    assert!(receiver.await.is_err());
}

#[tokio::test]
async fn pending_events_returns_both_kinds() {
    let registry = Arc::new(PendingRequestRegistry::new());
    let _elicitation_receiver = registry.register_elicitation(elicitation_params()).await;
    let _sampling_receiver = registry
        .register_sampling(sampling_params(), "test-server-x")
        .await;

    let mut kinds = registry
        .pending_events()
        .await
        .into_iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();
    kinds.sort_unstable();

    assert_eq!(kinds, vec!["elicitation", "sampling"]);
}
