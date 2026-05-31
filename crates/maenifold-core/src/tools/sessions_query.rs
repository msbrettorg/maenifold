use super::sessions::{AgentSessionError, ListedSession, NullAcpClient, SessionQueryResult};
use crate::sessions::extract_session_timestamp_millis;
use agent_client_protocol::schema::{self as acp};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use chrono::{TimeZone, Utc};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

static SESSIONS_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build sessions runtime: {error}"))
});

const MAX_PAGES: usize = 100;
const MAX_SESSIONS: usize = 10_000;

pub(super) fn run_sessions_future<F, T>(future: F) -> std::result::Result<T, String>
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

pub(super) async fn list_agent_sessions(
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
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        command.process_group(0);
    }
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

    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
    let null_client = Arc::new(NullAcpClient);
    let (connection_tx, connection_rx) = tokio::sync::oneshot::channel::<ConnectionTo<Agent>>();
    let io_future = Client
        .builder()
        .name("maenifold")
        .on_receive_request(
            {
                let null_client = null_client.clone();
                async move |req: acp::RequestPermissionRequest, responder, _connection| {
                    let response = null_client.request_permission(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let null_client = null_client.clone();
                async move |notif: acp::SessionNotification, _connection| {
                    null_client.session_notification(notif).await
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            transport,
            move |connection: ConnectionTo<Agent>| async move {
                connection_tx.send(connection).map_err(|_| {
                    acp::Error::new(
                        acp::ErrorCode::InternalError.into(),
                        "failed to receive ACP connection handle",
                    )
                })?;
                std::future::pending::<acp::Result<()>>().await
            },
        );
    let io_task = tokio::task::spawn_local(async move {
        let _ = io_future.await;
    });
    let connection = connection_rx
        .await
        .map_err(|_| format!("failed to receive ACP connection handle for agent '{agent_name}'"))?;

    let result = query_sessions(&connection, &agent_name).await;

    drop(connection);
    let _ = child.kill().await;
    let _ = child.wait().await;
    io_task.abort();

    result
}

async fn query_sessions(
    connection: &ConnectionTo<Agent>,
    agent_name: &str,
) -> std::result::Result<SessionQueryResult, String> {
    let init = connection
        .send_request(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(acp::Implementation::new("maenifold", "0.1.0").title("maenifold")),
        )
        .block_task()
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
            .send_request(request)
            .block_task()
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
