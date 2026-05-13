use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::mcp::resources::notify_resource_updated;

const SAMPLING_SESSIONS_URI: &str = "sampling://sessions";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SamplingDispatch {
    pub sampling_request_id: Uuid,
    pub app_session_id: Uuid,
    pub acp_session_id: Option<String>,
    pub server_name: String,
    pub model: String,
    pub runtime_kind: String,
    pub role_name: String,
    pub status: SamplingSessionStatus,
    pub created_at: String,
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum SamplingSessionStatus {
    PendingApproval,
    Dispatching,
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SamplingRegistryError {
    NotFound,
    AlreadyTerminal,
}

pub(crate) struct SamplingSessionRegistry {
    sessions: Arc<Mutex<HashMap<Uuid, SamplingDispatch>>>,
}

impl SamplingSessionRegistry {
    pub(crate) fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) async fn insert(&self, dispatch: SamplingDispatch) {
        self.sessions
            .lock()
            .await
            .insert(dispatch.sampling_request_id, dispatch);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
    }

    pub(crate) async fn update_status(
        &self,
        id: Uuid,
        status: SamplingSessionStatus,
    ) -> Result<(), SamplingRegistryError> {
        let mut sessions = self.sessions.lock().await;
        let Some(dispatch) = sessions.get_mut(&id) else {
            return Err(SamplingRegistryError::NotFound);
        };
        if matches!(
            dispatch.status,
            SamplingSessionStatus::Cancelled
                | SamplingSessionStatus::Failed
                | SamplingSessionStatus::Completed
        ) {
            return Err(SamplingRegistryError::AlreadyTerminal);
        }
        dispatch.status = status;
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn update_acp_session(
        &self,
        id: Uuid,
        acp_session_id: String,
    ) -> Result<(), SamplingRegistryError> {
        let mut sessions = self.sessions.lock().await;
        let Some(dispatch) = sessions.get_mut(&id) else {
            return Err(SamplingRegistryError::NotFound);
        };
        dispatch.acp_session_id = Some(acp_session_id);
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn record_usage(
        &self,
        id: Uuid,
        usage: TokenUsage,
    ) -> Result<(), SamplingRegistryError> {
        let mut sessions = self.sessions.lock().await;
        let Some(dispatch) = sessions.get_mut(&id) else {
            return Err(SamplingRegistryError::NotFound);
        };
        dispatch.token_usage = Some(usage);
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(())
    }

    pub(crate) async fn snapshot(&self) -> Vec<SamplingDispatch> {
        let mut sessions = self.sessions.lock().await.values().cloned().collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        sessions
    }

    #[allow(dead_code)]
    pub(crate) async fn remove(&self, id: Uuid) -> Option<SamplingDispatch> {
        let removed = self.sessions.lock().await.remove(&id);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        removed
    }
}

impl SamplingDispatch {
    pub(crate) fn runtime_kind(&self) -> &str {
        &self.runtime_kind
    }

    pub(crate) fn role_name(&self) -> &str {
        &self.role_name
    }
}

impl Default for SamplingSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
