use std::sync::{Arc, atomic::AtomicBool};

use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::mcp::sampling_session_registry::{
    SamplingDecisionRecord, SamplingDispatch, SamplingSessionRegistry, SamplingSessionStatus,
};

pub(crate) struct GatewayIntentObservation {
    id: Uuid,
    registry: Arc<SamplingSessionRegistry>,
}

pub(crate) async fn start(
    registry: Arc<SamplingSessionRegistry>,
    decision: SamplingDecisionRecord,
    runtime_kind: &str,
    model: &str,
    role_name: &str,
    app_session_id: Uuid,
    request_payload: String,
) -> GatewayIntentObservation {
    let id = Uuid::new_v4();
    registry
        .insert(SamplingDispatch {
            sampling_request_id: id,
            app_session_id,
            acp_session_id: None,
            server_name: "GatewayIntent".to_string(),
            model: model.to_string(),
            runtime_kind: runtime_kind.to_string(),
            role_name: role_name.to_string(),
            status: SamplingSessionStatus::Dispatching,
            decision,
            created_at: chrono::Utc::now().to_rfc3339(),
            token_usage: None,
            request_payload: Some(request_payload),
            response_payload: None,
            tool_call_count: 0,
            tool_loop_budget: 0,
            turn_count: 0,
            status_handle: Arc::new(AtomicBool::new(false)),
        })
        .await;
    GatewayIntentObservation { id, registry }
}

impl GatewayIntentObservation {
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> Uuid {
        self.id
    }

    #[allow(dead_code)]
    pub(crate) fn registry(&self) -> Arc<SamplingSessionRegistry> {
        Arc::clone(&self.registry)
    }

    pub(crate) async fn complete<T: Serialize>(&self, payload: &T) {
        self.finish(payload, SamplingSessionStatus::Completed).await;
    }

    pub(crate) async fn fail<T: Serialize>(&self, payload: &T) {
        self.finish(payload, SamplingSessionStatus::Failed).await;
    }

    async fn finish<T: Serialize>(&self, payload: &T, status: SamplingSessionStatus) {
        let _ = self
            .registry
            .record_response(self.id, pretty_payload(payload))
            .await;
        let _ = self.registry.update_status(self.id, status).await;
    }
}

pub(crate) fn pretty_payload<T: Serialize>(payload: &T) -> String {
    serde_json::to_string_pretty(payload)
        .unwrap_or_else(|err| json!({ "serializationError": err.to_string() }).to_string())
}
