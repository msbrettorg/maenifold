use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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
    pub decision: SamplingDecisionRecord,
    pub created_at: String,
    pub token_usage: Option<TokenUsage>,
    pub request_payload: Option<String>,
    pub response_payload: Option<String>,
    pub tool_call_count: u32,
    pub tool_loop_budget: u32,
    pub turn_count: u32,
    #[serde(skip)]
    pub status_handle: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) enum SamplingDecisionRecord {
    AllowOnce,
    AllowAlways,
    ClientSampling,
    Inferencing,
    Deny,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[allow(dead_code)]
pub(crate) enum SamplingSessionStatus {
    PendingApproval,
    Dispatching,
    Streaming,
    BudgetExceeded { allowed_so_far: u32, budget: u32 },
    Completed,
    Cancelled,
    Denied,
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
    default_tool_loop_budget: u32,
    per_server_tool_loop_budget: Arc<HashMap<String, u32>>,
}

impl SamplingSessionRegistry {
    pub(crate) fn new() -> Self {
        Self::with_tool_loop_budgets(10, HashMap::new())
    }

    pub(crate) fn with_tool_loop_budgets(
        default_tool_loop_budget: u32,
        per_server_tool_loop_budget: HashMap<String, u32>,
    ) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            default_tool_loop_budget,
            per_server_tool_loop_budget: Arc::new(per_server_tool_loop_budget),
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
                | SamplingSessionStatus::Denied
                | SamplingSessionStatus::Failed
                | SamplingSessionStatus::Completed
        ) {
            return Err(SamplingRegistryError::AlreadyTerminal);
        }
        dispatch
            .status_handle
            .store(is_terminal_status(&status), Ordering::SeqCst);
        dispatch.status = status;
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(())
    }

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

    pub(crate) async fn acp_session_id(&self, id: Uuid) -> Option<String> {
        self.sessions
            .lock()
            .await
            .get(&id)
            .and_then(|dispatch| dispatch.acp_session_id.clone())
    }

    pub(crate) async fn get(&self, id: Uuid) -> Option<SamplingDispatch> {
        self.sessions.lock().await.get(&id).cloned()
    }

    pub(crate) async fn status_handle(&self, id: Uuid) -> Option<Arc<AtomicBool>> {
        self.sessions
            .lock()
            .await
            .get(&id)
            .map(|dispatch| Arc::clone(&dispatch.status_handle))
    }

    pub(crate) fn tool_loop_budget_for(&self, server_name: &str) -> u32 {
        self.per_server_tool_loop_budget
            .get(server_name)
            .copied()
            .unwrap_or(self.default_tool_loop_budget)
    }

    pub(crate) async fn increment_turn(&self, dispatch_id: Uuid) -> Result<u32, String> {
        let mut sessions = self.sessions.lock().await;
        let dispatch = sessions
            .get_mut(&dispatch_id)
            .ok_or_else(|| "sampling dispatch not found".to_string())?;
        dispatch.turn_count = dispatch.turn_count.saturating_add(1);
        let count = dispatch.turn_count;
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(count)
    }

    pub(crate) async fn increment_tool_calls(
        &self,
        dispatch_id: Uuid,
        n: u32,
    ) -> Result<u32, String> {
        let mut sessions = self.sessions.lock().await;
        let dispatch = sessions
            .get_mut(&dispatch_id)
            .ok_or_else(|| "sampling dispatch not found".to_string())?;
        dispatch.tool_call_count = dispatch.tool_call_count.saturating_add(n);
        let count = dispatch.tool_call_count;
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(count)
    }

    pub(crate) async fn reset_tool_calls(&self, dispatch_id: Uuid) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        let dispatch = sessions
            .get_mut(&dispatch_id)
            .ok_or_else(|| "sampling dispatch not found".to_string())?;
        dispatch.tool_call_count = 0;
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

    pub(crate) async fn record_response(
        &self,
        id: Uuid,
        response_payload: String,
    ) -> Result<(), SamplingRegistryError> {
        let mut sessions = self.sessions.lock().await;
        let Some(dispatch) = sessions.get_mut(&id) else {
            return Err(SamplingRegistryError::NotFound);
        };
        dispatch.response_payload = Some(response_payload);
        drop(sessions);
        notify_resource_updated(SAMPLING_SESSIONS_URI).await;
        Ok(())
    }

    pub(crate) async fn snapshot(&self) -> Vec<SamplingDispatch> {
        let mut sessions = self
            .sessions
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
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

pub(crate) fn is_terminal_status(status: &SamplingSessionStatus) -> bool {
    matches!(
        status,
        SamplingSessionStatus::Cancelled
            | SamplingSessionStatus::Denied
            | SamplingSessionStatus::Failed
            | SamplingSessionStatus::Completed
    )
}

impl Default for SamplingSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
