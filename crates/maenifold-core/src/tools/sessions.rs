use super::context::{ToolContext, load_help_doc, render};
use crate::acp_config::AgentsConfig;
use crate::error::{MaenifoldError, Result};
use crate::sessions::extract_session_timestamp_millis;
use crate::util::json::{OutputFormat, success};
use agent_client_protocol::{self as acp, Agent as _};
use chrono::{TimeZone, Utc};
use futures_util::future::join_all;
use serde::Serialize;
use serde_json::{Value, json};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;
use tokio::time::{Duration, timeout};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

static SESSIONS_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build sessions runtime: {error}"))
});

const SESSION_LIST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PAGES: usize = 100;
const MAX_SESSIONS: usize = 10_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListedSession {
    session_id: String,
    agent_name: String,
    cwd: String,
    title: Option<String>,
    created_at: Option<String>,
    last_activity: Option<String>,
    updated_at: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionError {
    agent_name: String,
    error: String,
}

struct ListedAgentSessions {
    sessions: Vec<ListedSession>,
    errors: Vec<AgentSessionError>,
}

struct SessionQueryResult {
    sessions: Vec<ListedSession>,
}

struct NullAcpClient;

#[async_trait::async_trait(?Send)]
impl acp::Client for NullAcpClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(&self, _args: acp::SessionNotification) -> acp::Result<()> {
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
    let config = AgentsConfig::load(root);
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

fn run_sessions_future<F, T>(future: F) -> std::result::Result<T, String>
where
    F: Future<Output = std::result::Result<T, String>>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(
                    "session listing requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                );
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => sessions_runtime()?.block_on(future),
    }
}

fn sessions_runtime() -> std::result::Result<&'static Runtime, String> {
    SESSIONS_RUNTIME.as_ref().map_err(|message| message.clone())
}

async fn list_agent_sessions(
    root: PathBuf,
    agent_name: String,
) -> std::result::Result<SessionQueryResult, AgentSessionError> {
    let local = tokio::task::LocalSet::new();
    let agent_name_for_query = agent_name.clone();
    local
        .run_until(async move { list_agent_sessions_local(root, agent_name_for_query).await })
        .await
        .map_err(|error| AgentSessionError { agent_name, error })
}

async fn list_agent_sessions_local(
    root: PathBuf,
    agent_name: String,
) -> std::result::Result<SessionQueryResult, String> {
    let adapter = jockey::acp::build_stdio_adapter(&agent_name)?
        .ok_or_else(|| format!("agent '{agent_name}' is not configured"))?;
    let spawn_cwd = jockey::acp::resolve_cwd(root.to_string_lossy().as_ref());

    let mut command = tokio::process::Command::new(&adapter.binary);
    command
        .args(&adapter.args)
        .current_dir(&spawn_cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .process_group(0);
    for (key, value) in &adapter.env {
        command.env(key, value);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn agent '{agent_name}': {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("stdin unavailable for agent '{agent_name}'"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("stdout unavailable for agent '{agent_name}'"))?;

    let (connection, io_future) = acp::ClientSideConnection::new(
        NullAcpClient,
        stdin.compat_write(),
        stdout.compat(),
        |future| {
            tokio::task::spawn_local(future);
        },
    );
    let io_task = tokio::task::spawn_local(async move {
        let _ = io_future.await;
    });

    let result = query_sessions(&connection, &agent_name).await;

    drop(connection);
    let _ = child.kill().await;
    let _ = child.wait().await;
    io_task.abort();

    result
}

async fn query_sessions(
    connection: &acp::ClientSideConnection,
    agent_name: &str,
) -> std::result::Result<SessionQueryResult, String> {
    let init = connection
        .initialize(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(acp::Implementation::new("maenifold", "0.1.0").title("maenifold")),
        )
        .await
        .map_err(|error| format!("initialize failed for agent '{agent_name}': {error}"))?;

    let supports_load_session = init.agent_capabilities.load_session;
    if init.agent_capabilities.session_capabilities.list.is_none() {
        return Ok(SessionQueryResult {
            sessions: Vec::new(),
        });
    }

    let mut sessions = Vec::new();
    let mut cursor: Option<String> = None;
    let mut page_count = 0;
    loop {
        page_count += 1;
        if page_count > MAX_PAGES {
            break;
        }
        let mut request = acp::ListSessionsRequest::new();
        if let Some(cursor_token) = cursor.take() {
            request = request.cursor(cursor_token);
        }
        let response = connection
            .list_sessions(request)
            .await
            .map_err(|error| format!("session/list failed for agent '{agent_name}': {error}"))?;
        sessions.extend(
            response.sessions.into_iter().map(|session| {
                listed_session_from_agent(agent_name, supports_load_session, session)
            }),
        );
        if sessions.len() >= MAX_SESSIONS {
            sessions.truncate(MAX_SESSIONS);
            break;
        }
        cursor = response.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    Ok(SessionQueryResult { sessions })
}

fn listed_session_from_agent(
    agent_name: &str,
    supports_load_session: bool,
    session: acp::SessionInfo,
) -> ListedSession {
    let session_id = session.session_id.to_string();
    let created_at = extract_session_timestamp_millis(&session_id).and_then(timestamp_to_rfc3339);
    let last_activity = session.updated_at.clone();
    ListedSession {
        session_id,
        agent_name: agent_name.to_string(),
        cwd: session.cwd.to_string_lossy().into_owned(),
        title: session.title,
        created_at,
        updated_at: last_activity.clone(),
        last_activity,
        status: if supports_load_session {
            "resumable".to_string()
        } else {
            "listed".to_string()
        },
    }
}

fn timestamp_to_rfc3339(timestamp_ms: i64) -> Option<String> {
    Utc.timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|timestamp| timestamp.to_rfc3339())
}
