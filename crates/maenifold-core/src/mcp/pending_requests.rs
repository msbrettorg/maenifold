use crate::mcp::elicitation_validation::validate_elicitation_response;
use rmcp::model::{CreateElicitationRequestParams, ElicitationAction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast, oneshot};
use uuid::Uuid;

pub(crate) const ELICITATION_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const RENDERER_GRACE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserPromptEvent {
    pub(crate) id: Uuid,
    pub(crate) kind: &'static str,
    pub(crate) payload: Value,
    pub(crate) timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UserPromptAction {
    Accept,
    Cancel,
    Dismiss,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ElicitationResponse {
    pub(crate) action: UserPromptAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<Value>,
}

impl ElicitationResponse {
    pub(crate) fn to_mcp_action(&self) -> ElicitationAction {
        match self.action {
            UserPromptAction::Accept => ElicitationAction::Accept,
            UserPromptAction::Cancel => ElicitationAction::Cancel,
            UserPromptAction::Dismiss => ElicitationAction::Decline,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum RespondError {
    Gone,
    Invalid(Vec<String>),
}

struct PendingRequest {
    params: CreateElicitationRequestParams,
    event: UserPromptEvent,
    sender: oneshot::Sender<ElicitationResponse>,
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

    pub(crate) async fn register(
        self: &Arc<Self>,
        params: CreateElicitationRequestParams,
    ) -> oneshot::Receiver<ElicitationResponse> {
        let (sender, receiver) = oneshot::channel();
        let id = Uuid::new_v4();
        let event = UserPromptEvent {
            id,
            kind: "elicitation",
            payload: serde_json::to_value(&params).unwrap_or(Value::Null),
            timeout_ms: ELICITATION_TIMEOUT.as_millis() as u64,
        };
        self.pending.lock().await.insert(
            id,
            PendingRequest {
                params,
                event: event.clone(),
                sender,
            },
        );
        let _ = self.sender.send(event);
        self.spawn_timeout(id, ELICITATION_TIMEOUT);
        self.spawn_renderer_grace(id);
        receiver
    }

    pub(crate) async fn respond(
        &self,
        id: Uuid,
        response: ElicitationResponse,
    ) -> Result<(), RespondError> {
        let mut pending = self.pending.lock().await;
        let Some(request) = pending.get(&id) else {
            return Err(RespondError::Gone);
        };
        if response.action == UserPromptAction::Accept {
            let Some(content) = response.content.as_ref() else {
                return Err(RespondError::Invalid(vec![
                    "accepted elicitation response requires content".to_string(),
                ]));
            };
            if let Some(schema) = requested_schema_value(&request.params) {
                validate_elicitation_response(&schema, content).map_err(RespondError::Invalid)?;
            }
        }
        let request = pending.remove(&id).ok_or(RespondError::Gone)?;
        request.sender.send(response).map_err(|_| RespondError::Gone)
    }

    async fn expire(&self, id: Uuid) {
        self.pending.lock().await.remove(&id);
    }

    fn spawn_timeout(self: &Arc<Self>, id: Uuid, timeout: Duration) {
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            registry.expire(id).await;
        });
    }

    fn spawn_renderer_grace(self: &Arc<Self>, id: Uuid) {
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(RENDERER_GRACE).await;
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

fn requested_schema_value(params: &CreateElicitationRequestParams) -> Option<Value> {
    match params {
        CreateElicitationRequestParams::FormElicitationParams {
            requested_schema, ..
        } => serde_json::to_value(requested_schema).ok(),
        CreateElicitationRequestParams::UrlElicitationParams { .. } => None,
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
        let _receiver = registry.register(params()).await;

        assert_eq!(registry.pending_events().await.len(), 1);
    }

    #[tokio::test]
    async fn subscribe_before_snapshot_does_not_lose_concurrent_register() {
        let registry = Arc::new(PendingRequestRegistry::new());
        let mut live_events = registry.subscribe();
        let register_registry = Arc::clone(&registry);
        let register_task = tokio::spawn(async move {
            let _receiver = register_registry.register(params()).await;
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
        let receiver = registry.register(params()).await;
        let id = registry.pending_events().await[0].id;
        let response = ElicitationResponse {
            action: UserPromptAction::Cancel,
            content: None,
        };

        registry
            .respond(id, response.clone())
            .await
            .expect("first response should resolve");
        assert_eq!(receiver.await.expect("response delivered"), response);
        assert_eq!(
            registry.respond(id, response).await,
            Err(RespondError::Gone)
        );
    }
}
