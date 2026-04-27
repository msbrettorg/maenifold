use crate::acp_config::TeamsMonitorConfig;
use crate::teams::TeamsRuntimeState;
use serde_json::{Value, json};

pub(super) fn status_value(
    action: &str,
    monitor: &TeamsMonitorConfig,
    runtime: &TeamsRuntimeState,
    message: &str,
) -> Value {
    let mut value = runtime.public_status_value();
    if let Value::Object(map) = &mut value {
        map.insert("action".to_string(), json!(action));
        map.insert(
            "config".to_string(),
            json!({
                "identity": monitor.identity,
                "keyword": monitor.keyword,
                "listenScope": monitor.listen_scope,
                "monitoredConversations": monitor.monitored_conversations,
                "replyBehavior": monitor.reply_behavior,
                "replyPrefix": monitor.reply_prefix,
                "dispatchTimeoutMinutes": monitor.dispatch_timeout_minutes,
                "dispatchAgent": monitor.dispatch_agent,
                "connection": monitor.connection,
            }),
        );
        map.insert("message".to_string(), json!(message));
    }
    value
}
