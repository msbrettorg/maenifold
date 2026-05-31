use agent_client_protocol::schema::{self as acp};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use serde_json::{Value, json};
use std::io::ErrorKind;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::super::adapter::{acp_log, clip};
use super::super::client::JockeyUiClient;
use super::super::runtime_state::{
    remember_agent_capabilities, remember_runtime_config_options, remember_runtime_models,
};
use super::super::worker::{AcpEvent, DeltaSlot, LiveConnection, register_child_pid};
use super::cold_start_helpers::{
    INIT_TIMEOUT, LIVE_CONNECTION_SEQ, LOAD_SESSION_TIMEOUT, SESSION_TIMEOUT,
    extract_models_from_config_options,
};

pub(crate) use super::cold_start_helpers::default_client_capabilities;

pub(crate) async fn cold_start(
    runtime_key: &str,
    _role_name: &str,
    _app_session_id: &str,
    binary: &str,
    args: &[String],
    env_pairs: &[(String, String)],
    client_capabilities: acp::ClientCapabilities,
    abs_cwd: &str,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    initial_delta_tx: Option<mpsc::Sender<AcpEvent>>,
    resume_session_id: Option<String>,
) -> Result<LiveConnection, String> {
    let delta_slot: DeltaSlot = Arc::new(Mutex::new(initial_delta_tx));
    acp_log("spawn.start", json!({ "binary": binary, "cwd": abs_cwd }));

    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args)
        .current_dir(abs_cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    for (k, v) in env_pairs {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        ErrorKind::NotFound => format!("Binary not found: {binary}"),
        _ => format!("Failed to start agent '{runtime_key}': {e}"),
    })?;
    if let Some(pid) = child.id() {
        register_child_pid(pid);
    }
    let stdin = child.stdin.take().ok_or("stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("stdout unavailable")?;

    if let Some(stderr) = child.stderr.take() {
        let bin = binary.to_string();
        tokio::task::spawn_local(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut r = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match r.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) if !line.trim().is_empty() => acp_log(
                        "stderr",
                        json!({ "binary": bin, "line": clip(line.trim(), 360) }),
                    ),
                    _ => {}
                }
            }
        });
    }

    acp_log("spawn.ok", json!({ "binary": binary, "pid": child.id() }));

    let (health_tx, health_rx) = tokio::sync::watch::channel(true);
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
    let ui_client = Arc::new(JockeyUiClient {
        delta_slot: delta_slot.clone(),
        auto_approve,
        workspace_root: std::path::PathBuf::from(abs_cwd),
    });
    let (connection_tx, connection_rx) = tokio::sync::oneshot::channel::<ConnectionTo<Agent>>();
    let connection_future = Client
        .builder()
        .name("jockey")
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::RequestPermissionRequest, responder, _connection| {
                    let response = ui_client.request_permission(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::ReadTextFileRequest, responder, _connection| {
                    let response = ui_client.read_text_file(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::WriteTextFileRequest, responder, _connection| {
                    let response = ui_client.write_text_file(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::CreateTerminalRequest, responder, _connection| {
                    let response = ui_client.create_terminal(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::TerminalOutputRequest, responder, _connection| {
                    let response = ui_client.terminal_output(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::WaitForTerminalExitRequest, responder, _connection| {
                    let response = ui_client.wait_for_terminal_exit(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::KillTerminalRequest, responder, _connection| {
                    let response = ui_client.kill_terminal(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::ReleaseTerminalRequest, responder, _connection| {
                    let response = ui_client.release_terminal(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let ui_client = ui_client.clone();
                async move |notif: acp::SessionNotification, _connection| {
                    ui_client.session_notification(notif).await
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
    let io_health_tx = health_tx.clone();
    let io_handle = tokio::task::spawn_local(async move {
        if let Err(e) = connection_future.await {
            acp_log("io_task.error", json!({ "error": e.to_string() }));
        }
        let _ = io_health_tx.send(false);
    });
    let conn = connection_rx
        .await
        .map_err(|_| "failed to receive ACP connection handle".to_string())?;

    let wait_health_tx = health_tx.clone();
    let wait_pid = child.id();
    // T-328.14: explicit process-exit watcher. The io_future-driven health
    // signal isn't sufficient — a child can be SIGKILLed and linger as a zombie
    // (or its stdout EOF can be delayed) for many minutes before the io task
    // notices. Owning the Child here and awaiting wait() surfaces death within
    // a tokio tick of reap.
    tokio::task::spawn_local(async move {
        let status = child.wait().await;
        acp_log(
            "child.exited",
            json!({
                "pid": wait_pid,
                "status": match &status {
                    Ok(s) => format!("{s}"),
                    Err(e) => e.to_string(),
                }
            }),
        );
        let _ = wait_health_tx.send(false);
    });
    let t = Instant::now();
    let init_resp = tokio::time::timeout(
        INIT_TIMEOUT,
        conn.send_request(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(acp::Implementation::new("jockey", "0.1.0").title("Jockey"))
                .client_capabilities(client_capabilities),
        )
        .block_task(),
    )
    .await
    .map_err(|_| format!("timeout: initialize exceeded {}s", INIT_TIMEOUT.as_secs()))?
    .map_err(|e| e.to_string())?;
    acp_log(
        "stage.ok",
        json!({ "stage": "initialize", "latencyMs": t.elapsed().as_millis() }),
    );

    let t = Instant::now();
    if let Ok(capabilities) = serde_json::to_value(&init_resp.agent_capabilities) {
        remember_agent_capabilities(runtime_key, capabilities);
    }
    let supports_load = init_resp.agent_capabilities.load_session;

    struct SessionStartResult {
        session_id: acp::SessionId,
        config_options: Option<Vec<acp::SessionConfigOption>>,
        resumed: bool,
    }

    async fn do_new_session(
        conn: &ConnectionTo<Agent>,
        abs_cwd: &str,
        mcp_servers: Vec<acp::McpServer>,
    ) -> Result<SessionStartResult, String> {
        let mut req = acp::NewSessionRequest::new(std::path::PathBuf::from(abs_cwd));
        if !mcp_servers.is_empty() {
            req = req.mcp_servers(mcp_servers);
        }
        let resp = tokio::time::timeout(SESSION_TIMEOUT, conn.send_request(req).block_task())
            .await
            .map_err(|_| {
                format!(
                    "timeout: new_session exceeded {}s",
                    SESSION_TIMEOUT.as_secs()
                )
            })?
            .map_err(|e| e.to_string())?;
        Ok(SessionStartResult {
            session_id: resp.session_id,
            config_options: resp.config_options,
            resumed: false,
        })
    }

    let start_result: SessionStartResult = if let Some(sid) = resume_session_id {
        if !supports_load {
            return Err(format!(
                "agent '{_role_name}' does not advertise loadSession capability; cannot resume session '{sid}'"
            ));
        }
        {
            let mut load_req = acp::LoadSessionRequest::new(
                acp::SessionId::from(sid.clone()),
                std::path::PathBuf::from(abs_cwd),
            );
            if !mcp_servers.is_empty() {
                load_req = load_req.mcp_servers(mcp_servers.clone());
            }
            match tokio::time::timeout(
                LOAD_SESSION_TIMEOUT,
                conn.send_request(load_req).block_task(),
            )
            .await
            .map_err(|_| {
                format!(
                    "timeout: load_session exceeded {}s",
                    LOAD_SESSION_TIMEOUT.as_secs()
                )
            })
            .and_then(|r| r.map_err(|e| e.to_string()))
            {
                Ok(resp) => {
                    acp_log(
                        "session.load.ok",
                        json!({ "runtime": runtime_key, "sessionId": &sid }),
                    );
                    SessionStartResult {
                        session_id: acp::SessionId::from(sid),
                        config_options: resp.config_options,
                        resumed: true,
                    }
                }
                Err(e) => {
                    acp_log(
                        "session.load.error",
                        json!({ "runtime": runtime_key, "sessionId": &sid, "error": e.to_string() }),
                    );
                    return Err(format!(
                        "session/load failed for session '{sid}' on agent '{_role_name}': {e}"
                    ));
                }
            }
        }
    } else {
        do_new_session(&conn, abs_cwd, mcp_servers).await?
    };

    let discovered_models =
        extract_models_from_config_options(start_result.config_options.as_ref());
    remember_runtime_models(runtime_key, discovered_models.clone());
    if let Some(ref opts) = start_result.config_options {
        let serialized: Vec<Value> = opts
            .iter()
            .filter_map(|o| serde_json::to_value(o).ok())
            .collect();
        acp_log(
            "config_options.discovered",
            json!({
                "runtime": runtime_key,
                "count": serialized.len(),
                "ids": serialized.iter().filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string())).collect::<Vec<_>>()
            }),
        );
        remember_runtime_config_options(runtime_key, serialized);
    } else {
        acp_log("config_options.none", json!({ "runtime": runtime_key }));
    }
    let session_id = start_result.session_id;

    acp_log(
        "stage.ok",
        json!({
            "stage": if start_result.resumed { "session/load" } else { "session/new" },
            "latencyMs": t.elapsed().as_millis(),
            "sessionId": session_id.to_string(),
            "runtime": runtime_key,
            "discoveredModelCount": discovered_models.len(),
            "resumed": start_result.resumed
        }),
    );

    Ok(LiveConnection {
        instance_id: LIVE_CONNECTION_SEQ.fetch_add(1, Ordering::Relaxed),
        conn: Rc::new(conn),
        session_id,
        resumed: start_result.resumed,
        cwd: abs_cwd.to_string(),
        delta_slot,
        child_pid: wait_pid,
        _io_task: io_handle,
        health_rx,
    })
}
