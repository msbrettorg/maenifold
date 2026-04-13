use agent_client_protocol as acp;
use dashmap::DashMap;
use std::sync::OnceLock;
use tokio::sync::oneshot;

static PERMISSION_REQUESTS: OnceLock<
    DashMap<String, oneshot::Sender<acp::RequestPermissionOutcome>>,
> = OnceLock::new();

pub(crate) fn permission_requests(
) -> &'static DashMap<String, oneshot::Sender<acp::RequestPermissionOutcome>> {
    PERMISSION_REQUESTS.get_or_init(DashMap::new)
}

pub fn respond_to_permission(request_id: &str, outcome: acp::RequestPermissionOutcome) {
    if let Some((_, tx)) = permission_requests().remove(request_id) {
        let _ = tx.send(outcome);
    }
}
