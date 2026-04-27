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
        "authState": runtime.auth_state.as_str(),
        "expiresAt": runtime.status_expires_at(),
        "lastAuthError": runtime.last_auth_error.as_deref(),
        "config": {
            "identity": monitor.identity,
            "keyword": monitor.keyword,
            "listenScope": monitor.listen_scope,
            "monitoredConversations": monitor.monitored_conversations,
            "replyBehavior": monitor.reply_behavior,
            "replyPrefix": monitor.reply_prefix,
            "dispatchTimeoutMinutes": monitor.dispatch_timeout_minutes,
            "dispatchAgent": monitor.dispatch_agent,
            "connection": monitor.connection,
        },
        "message": message,
    })
}
