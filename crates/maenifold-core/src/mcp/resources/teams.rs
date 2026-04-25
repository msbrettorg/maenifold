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
        TEAMS_STATUS_URI => {
            let state = teams_runtime_snapshot();
            json!({
                "enabled": state.enabled(),
                "connectionState": state.connection_state,
                "lastMessageAt": state.last_message_at,
                "lastMatchAt": state.last_match_at,
                "matchCount": state.match_count,
                "activeDispatches": state.active_dispatches,
                "lastError": state.last_error,
            })
        }
        TEAMS_CONFIG_URI => {
            let config = Config::current().settings.teams_monitor;
            json!({
                "keywords": config.handler.keywords,
                "senderAllowList": config.handler.sender_allow_list,
                "senderDenyList": config.handler.sender_deny_list,
                "cooldownSeconds": config.handler.cooldown_seconds,
                "dispatchAgent": config.dispatch_agent,
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
