use super::context::{ToolContext, load_help_doc, render};
use super::sessions_query::{list_agent_sessions, run_sessions_future};
use crate::acp_config::AgentsConfig;
use crate::error::{MaenifoldError, Result};
use crate::util::json::{OutputFormat, success};
use agent_client_protocol::schema::{self as acp};
use futures_util::future::join_all;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use tokio::time::{Duration, timeout};

const SESSION_LIST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListedSession {
    pub(super) session_id: String,
    pub(super) agent_name: String,
    pub(super) cwd: String,
    pub(super) title: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) last_activity: Option<String>,
    pub(super) updated_at: Option<String>,
    pub(super) status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AgentSessionError {
    pub(super) agent_name: String,
    pub(super) error: String,
}

pub(super) struct ListedAgentSessions {
    pub(super) sessions: Vec<ListedSession>,
    pub(super) errors: Vec<AgentSessionError>,
}

pub(super) struct SessionQueryResult {
    pub(super) sessions: Vec<ListedSession>,
}

pub(super) struct NullAcpClient;

impl NullAcpClient {
    pub(crate) async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Cancelled,
        ))
    }

    pub(crate) async fn session_notification(
        &self,
        _args: acp::SessionNotification,
    ) -> acp::Result<()> {
        Ok(())
    }
}

pub fn list_sessions(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "ListSessions")
            .unwrap_or_else(|| "ERROR: Help file not found for ListSessions".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let listed = list_sessions_value(&context.config.maenifold_root)?;
    Ok(render(
        context.output_format,
        json!({
            "sessions": listed.sessions,
            "errors": listed.errors,
        }),
    ))
}

fn list_sessions_value(root: &Path) -> Result<ListedAgentSessions> {
    let config = AgentsConfig::load(root)?;
    let mut agent_names = config.agents.keys().cloned().collect::<Vec<_>>();
    agent_names.sort_unstable();
    jockey::acp::set_config_root(root.to_path_buf());
    let root = root.to_path_buf();

    run_sessions_future(async move {
        let results = join_all(agent_names.into_iter().map(|agent_name| {
            let root = root.clone();
            async move {
                let timeout_agent_name = agent_name.clone();
                match timeout(SESSION_LIST_TIMEOUT, list_agent_sessions(root, agent_name)).await {
                    Ok(result) => result,
                    Err(_) => Err(AgentSessionError {
                        agent_name: timeout_agent_name,
                        error: "Timed out after 10s".to_string(),
                    }),
                }
            }
        }))
        .await;

        let mut sessions = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(mut agent_sessions) => sessions.append(&mut agent_sessions.sessions),
                Err(error) => errors.push(error),
            }
        }

        sessions.sort_by(|left, right| {
            right
                .last_activity
                .cmp(&left.last_activity)
                .then_with(|| right.created_at.cmp(&left.created_at))
                .then_with(|| left.agent_name.cmp(&right.agent_name))
                .then_with(|| left.session_id.cmp(&right.session_id))
        });

        Ok(ListedAgentSessions { sessions, errors })
    })
    .map_err(MaenifoldError::Message)
}
