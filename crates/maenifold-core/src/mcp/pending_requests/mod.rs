mod elicitation;
mod sampling;

use elicitation::validate_response_content;
pub(crate) use elicitation::{ElicitationResponse, UserPromptAction};
#[allow(unused_imports)]
pub(crate) use sampling::SamplingAction;
pub(crate) use sampling::SamplingDecision;

use rmcp::model::CreateElicitationRequestParams;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast, oneshot};
use uuid::Uuid;

pub(crate) const ELICITATION_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const RENDERER_GRACE: Duration = Duration::from_secs(5);
pub(crate) const SAMPLING_BACKSTOP_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserPromptEvent {
    pub(crate) id: Uuid,
    pub(crate) kind: &'static str,
    pub(crate) payload: Value,
    pub(crate) timeout_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum UserPromptResponse {
    Elicitation {
        id: Uuid,
        response: ElicitationResponse,
    },
    Sampling {
        id: Uuid,
        response: SamplingDecision,
    },
}

#[derive(Debug, PartialEq)]
pub(crate) enum RespondError {
    Gone,
    Invalid(Vec<String>),
    KindMismatch,
}

pub(crate) enum PendingPayload {
    Elicitation {
        params: CreateElicitationRequestParams,
        sender: oneshot::Sender<ElicitationResponse>,
    },
    #[allow(dead_code)]
    Sampling {
        params: rmcp::model::CreateMessageRequestParams,
        sender: oneshot::Sender<SamplingDecision>,
    },
}

struct PendingRequest {
    payload: PendingPayload,
    event: UserPromptEvent,
}

pub struct PendingRequestRegistry {
    pending: Mutex<HashMap<Uuid, PendingRequest>>,
    sender: broadcast::Sender<UserPromptEvent>,
}

impl PendingRequestRegistry {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(32);
        Self {
            pending: Mutex::new(HashMap::new()),
            sender,
        }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<UserPromptEvent> {
        self.sender.subscribe()
    }

    pub(crate) async fn pending_events(&self) -> Vec<UserPromptEvent> {
        self.pending
            .lock()
            .await
            .values()
            .map(|request| request.event.clone())
            .collect()
    }

    pub(crate) async fn register_elicitation(
        self: &Arc<Self>,
        params: CreateElicitationRequestParams,
    ) -> oneshot::Receiver<ElicitationResponse> {
        let event = UserPromptEvent {
            id: Uuid::new_v4(),
            kind: "elicitation",
            payload: serde_json::to_value(&params).unwrap_or(Value::Null),
            timeout_ms: ELICITATION_TIMEOUT.as_millis() as u64,
        };
        let (sender, receiver) = oneshot::channel();
        self.insert(event, PendingPayload::Elicitation { params, sender })
            .await;
        receiver
    }

    #[allow(dead_code)]
    pub(crate) async fn register_sampling(
        self: &Arc<Self>,
        params: rmcp::model::CreateMessageRequestParams,
        server_name: &str,
    ) -> oneshot::Receiver<SamplingDecision> {
        let mut payload = serde_json::to_value(&params).unwrap_or(Value::Null);
        if let Value::Object(fields) = &mut payload {
            fields.insert(
                "serverName".to_string(),
                Value::String(server_name.to_string()),
            );
        }
        let event = UserPromptEvent {
            id: Uuid::new_v4(),
            kind: "sampling",
            payload,
            timeout_ms: SAMPLING_BACKSTOP_TIMEOUT.as_millis() as u64,
        };
        let (sender, receiver) = oneshot::channel();
        self.insert(event, PendingPayload::Sampling { params, sender })
            .await;
        receiver
    }

    pub(crate) async fn respond_typed(
        &self,
        response: UserPromptResponse,
    ) -> Result<(), RespondError> {
        let id = match &response {
            UserPromptResponse::Elicitation { id, .. }
            | UserPromptResponse::Sampling { id, .. } => *id,
        };
        let mut pending = self.pending.lock().await;
        let Some(request) = pending.get(&id) else {
            return Err(RespondError::Gone);
        };
        match (&request.payload, &response) {
            (PendingPayload::Elicitation { .. }, UserPromptResponse::Elicitation { .. }) => {}
            (PendingPayload::Sampling { .. }, UserPromptResponse::Sampling { .. }) => {}
            _ => return Err(RespondError::KindMismatch),
        }
        match response {
            UserPromptResponse::Elicitation { id, response } => {
                if response.action == UserPromptAction::Accept {
                    let Some(content) = response.content.as_ref() else {
                        return Err(RespondError::Invalid(vec![
                            "accepted elicitation response requires content".to_string(),
                        ]));
                    };
                    if let PendingPayload::Elicitation { params, .. } = &request.payload {
                        validate_response_content(params, content)
                            .map_err(RespondError::Invalid)?;
                    }
                }
                let request = pending.remove(&id).ok_or(RespondError::Gone)?;
                let PendingPayload::Elicitation { sender, .. } = request.payload else {
                    return Err(RespondError::KindMismatch);
                };
                sender.send(response).map_err(|_| RespondError::Gone)
            }
            UserPromptResponse::Sampling { id, response } => {
                let request = pending.remove(&id).ok_or(RespondError::Gone)?;
                let PendingPayload::Sampling { sender, .. } = request.payload else {
                    return Err(RespondError::KindMismatch);
                };
                sender.send(response).map_err(|_| RespondError::Gone)
            }
        }
    }

    async fn insert(self: &Arc<Self>, event: UserPromptEvent, payload: PendingPayload) {
        let id = event.id;
        let timeout = match &payload {
            PendingPayload::Elicitation { .. } => ELICITATION_TIMEOUT,
            PendingPayload::Sampling { .. } => SAMPLING_BACKSTOP_TIMEOUT,
        };
        let renderer_grace = matches!(payload, PendingPayload::Elicitation { .. });
        self.pending.lock().await.insert(
            id,
            PendingRequest {
                payload,
                event: event.clone(),
            },
        );
        let _ = self.sender.send(event);
        self.spawn_timeout(id, timeout);
        if renderer_grace {
            self.spawn_renderer_grace(id);
        }
    }

    async fn expire(&self, id: Uuid) {
        self.pending.lock().await.remove(&id);
    }

    fn spawn_timeout(self: &Arc<Self>, id: Uuid, timeout: Duration) {
        let registry = Arc::clone(self);
        let deadline = tokio::time::Instant::now() + timeout;
        tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            registry.expire(id).await;
        });
    }

    fn spawn_renderer_grace(self: &Arc<Self>, id: Uuid) {
        let registry = Arc::clone(self);
        let deadline = tokio::time::Instant::now() + RENDERER_GRACE;
        tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            if registry.sender.receiver_count() == 0 {
                registry.expire(id).await;
            }
        });
    }
}

impl Default for PendingRequestRegistry {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
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
}
