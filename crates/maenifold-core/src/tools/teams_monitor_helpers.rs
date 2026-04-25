use crate::acp_config::TeamsMonitorConfig;
use crate::teams::TeamsRuntimeState;
use serde_json::{Value, json};

pub(super) fn status_value(
    action: &str,
    monitor: &TeamsMonitorConfig,
    runtime: &TeamsRuntimeState,
    message: &str,
) -> Value {
    json!({
        "action": action,
        "enabled": runtime.enabled(),
        "connectionState": runtime.connection_state.as_str(),
        "lastMessageAt": runtime.last_message_at.as_deref(),
        "lastMatchAt": runtime.last_match_at.as_deref(),
        "matchCount": runtime.match_count,
        "activeDispatches": runtime.active_dispatches,
        "lastError": runtime.last_error.as_deref(),
        "config": {
            "clientId": monitor.client_id,
            "tenantId": monitor.tenant_id,
            "keywords": monitor.handler.keywords,
            "senderAllowList": monitor.handler.sender_allow_list,
            "senderDenyList": monitor.handler.sender_deny_list,
            "cooldownSeconds": monitor.handler.cooldown_seconds,
            "dispatchAgent": monitor.dispatch_agent,
        },
        "message": message,
    })
}
