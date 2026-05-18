mod elicitation;
#[cfg(test)]
mod tests;

use elicitation::validate_response_content;
pub(crate) use elicitation::{ElicitationResponse, UserPromptAction};

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
}

#[derive(Debug, PartialEq)]
pub(crate) enum RespondError {
    Gone,
    Invalid(Vec<String>),
    #[allow(dead_code)]
    KindMismatch,
}

pub(crate) enum PendingPayload {
    Elicitation {
        params: CreateElicitationRequestParams,
        sender: oneshot::Sender<ElicitationResponse>,
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

    pub(crate) async fn respond_typed(
        &self,
        response: UserPromptResponse,
    ) -> Result<(), RespondError> {
        let id = match &response {
            UserPromptResponse::Elicitation { id, .. } => *id,
        };
        let mut pending = self.pending.lock().await;
        let Some(request) = pending.get(&id) else {
            return Err(RespondError::Gone);
        };
        match (&request.payload, &response) {
            (PendingPayload::Elicitation { .. }, UserPromptResponse::Elicitation { .. }) => {}
        }
        match response {
            UserPromptResponse::Elicitation { id, response } => {
                if response.action == UserPromptAction::Accept {
                    let Some(content) = response.content.as_ref() else {
                        return Err(RespondError::Invalid(vec![
                            "accepted elicitation response requires content".to_string(),
                        ]));
                    };
                    let PendingPayload::Elicitation { params, .. } = &request.payload;
                    validate_response_content(params, content)
                        .map_err(RespondError::Invalid)?;
                }
                let request = pending.remove(&id).ok_or(RespondError::Gone)?;
                let PendingPayload::Elicitation { sender, .. } = request.payload;
                sender.send(response).map_err(|_| RespondError::Gone)
            }
        }
    }

    async fn insert(self: &Arc<Self>, event: UserPromptEvent, payload: PendingPayload) {
        let id = event.id;
        let timeout = match &payload {
            PendingPayload::Elicitation { .. } => ELICITATION_TIMEOUT,
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
