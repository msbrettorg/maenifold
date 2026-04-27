use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::teams::{TEAMS_STATUS_URI, teams_runtime_snapshot};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use serde_json::json;

const TEAMS_CONFIG_URI: &str = "teams://config";

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        make_resource(
            TEAMS_STATUS_URI,
            "Teams Monitor Status",
            "Current enabled and connection state for the Teams monitor.",
        ),
        make_resource(
            TEAMS_CONFIG_URI,
            "Teams Monitor Config",
            "Current Teams monitor routing and filter configuration.",
        ),
    ]
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let payload = match uri {
        TEAMS_STATUS_URI => teams_runtime_snapshot().public_status_value(),
        TEAMS_CONFIG_URI => {
            let cfg = Config::current();
            let config = cfg.settings.teams_monitor;
            let root = cfg.maenifold_root.to_string_lossy();
            json!({
                "maenifoldRoot": root.as_ref(),
                "sessionId": crate::teams::supervisor::TEAMS_SESSION_ID,
                "identity": config.identity,
                "keyword": config.keyword,
                "listenScope": config.listen_scope,
                "monitoredConversations": config.monitored_conversations,
                "replyBehavior": config.reply_behavior,
                "replyPrefix": config.reply_prefix,
                "dispatchTimeoutMinutes": config.dispatch_timeout_minutes,
                "dispatchAgent": config.dispatch_agent,
                "connection": config.connection,
            })
        }
        _ => {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected '{TEAMS_STATUS_URI}' or '{TEAMS_CONFIG_URI}'"
            )));
        }
    };

    let text = serde_json::to_string_pretty(&payload)?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn make_resource(uri: &str, name: &str, description: &str) -> Resource {
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    resource.title = Some(name.to_string());
    resource.description = Some(description.to_string());
    resource
}
