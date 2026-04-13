use agent_client_protocol::{self as acp, Agent as _};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::super::adapter::{acp_log, clip};
use super::super::client::JockeyUiClient;
use super::super::runtime_state::{
    remember_runtime_config_options, remember_runtime_models, remember_runtime_modes,
};
use super::super::worker::{register_child_pid, DeltaSlot, LiveConnection};

pub(super) const INIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub(super) const SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub(super) static LIVE_CONNECTION_SEQ: AtomicU64 = AtomicU64::new(1);

pub(super) fn collect_models_from_select_options(
    options: &acp::SessionConfigSelectOptions,
    out: &mut HashSet<String>,
) {
    match options {
        acp::SessionConfigSelectOptions::Ungrouped(items) => {
            for item in items {
                out.insert(item.value.to_string());
            }
        }
        acp::SessionConfigSelectOptions::Grouped(groups) => {
            for group in groups {
                for item in &group.options {
                    out.insert(item.value.to_string());
                }
            }
        }
        _ => {}
    }
}

pub(super) fn extract_models_from_config_options(
    options: Option<&Vec<acp::SessionConfigOption>>,
) -> Vec<String> {
    let Some(options) = options else {
        return Vec::new();
    };
    let mut out = HashSet::new();
    for option in options {
        let is_model_category = matches!(
            option.category,
            Some(acp::SessionConfigOptionCategory::Model)
        );
        let is_model_id = option.id.to_string().eq_ignore_ascii_case("model");
        if !(is_model_category || is_model_id) {
            continue;
        }
        if let acp::SessionConfigKind::Select(select) = &option.kind {
            out.insert(select.current_value.to_string());
            collect_models_from_select_options(&select.options, &mut out);
        }
    }
    let mut models = out.into_iter().collect::<Vec<_>>();
    models.sort_unstable();
    models
}

pub(super) fn extract_mode_ids(modes: &[Value]) -> Vec<String> {
    let mut out = HashSet::new();
    for mode in modes {
        if let Some(id) = mode.get("id").and_then(|v| v.as_str()) {
            out.insert(id.to_string());
            continue;
        }
        if let Some(id) = mode.as_str() {
            out.insert(id.to_string());
        }
    }
    let mut items = out.into_iter().collect::<Vec<_>>();
    items.sort_unstable();
    items
}

pub(crate) async fn cold_start(
    runtime_key: &'static str,
    _role_name: &str,
    _app_session_id: &str,
    binary: &str,
    args: &[String],
    env_pairs: &[(String, String)],
    abs_cwd: &str,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    resume_session_id: Option<String>,
) -> Result<LiveConnection, String> {
    let delta_slot: DeltaSlot = Arc::new(Mutex::new(None));
    acp_log("spawn.start", json!({ "binary": binary, "cwd": abs_cwd }));

    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args)
        .current_dir(abs_cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .process_group(0);
    for (k, v) in env_pairs {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
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

    let (conn, io_future) = acp::ClientSideConnection::new(
        JockeyUiClient {
            delta_slot: delta_slot.clone(),
            auto_approve,
        },
        stdin.compat_write(),
        stdout.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    let (health_tx, health_rx) = tokio::sync::watch::channel(true);
    let io_handle = tokio::task::spawn_local(async move {
        if let Err(e) = io_future.await {
            acp_log("io_task.error", json!({ "error": e.to_string() }));
        }
        let _ = health_tx.send(false);
    });

    let t = Instant::now();
    let init_resp = tokio::time::timeout(
        INIT_TIMEOUT,
        conn.initialize(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(acp::Implementation::new("jockey", "0.1.0").title("Jockey"))
                .client_capabilities(
                    acp::ClientCapabilities::new()
                        .fs(acp::FileSystemCapabilities::new()
                            .read_text_file(true)
                            .write_text_file(true))
                        .terminal(true),
                ),
        ),
    )
    .await
    .map_err(|_| format!("timeout: initialize exceeded {}s", INIT_TIMEOUT.as_secs()))?
    .map_err(|e| e.to_string())?;
    acp_log(
        "stage.ok",
        json!({ "stage": "initialize", "latencyMs": t.elapsed().as_millis() }),
    );

    let t = Instant::now();
    let supports_load = init_resp.agent_capabilities.load_session;

    struct SessionStartResult {
        session_id: acp::SessionId,
        config_options: Option<Vec<acp::SessionConfigOption>>,
        modes: Option<acp::SessionModeState>,
        resumed: bool,
    }

    async fn do_new_session(
        conn: &acp::ClientSideConnection,
        abs_cwd: &str,
        mcp_servers: Vec<acp::McpServer>,
    ) -> Result<SessionStartResult, String> {
        let mut req = acp::NewSessionRequest::new(std::path::PathBuf::from(abs_cwd));
        if !mcp_servers.is_empty() {
            req = req.mcp_servers(mcp_servers);
        }
        let resp = tokio::time::timeout(SESSION_TIMEOUT, conn.new_session(req))
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
            modes: resp.modes,
            resumed: false,
        })
    }

    let start_result: SessionStartResult = if supports_load {
        if let Some(sid) = resume_session_id {
            let mut load_req = acp::LoadSessionRequest::new(
                acp::SessionId::from(sid.clone()),
                std::path::PathBuf::from(abs_cwd),
            );
            if !mcp_servers.is_empty() {
                load_req = load_req.mcp_servers(mcp_servers.clone());
            }
            match tokio::time::timeout(SESSION_TIMEOUT, conn.load_session(load_req))
                .await
                .map_err(|_| {
                    acp::Error::new(
                        acp::ErrorCode::InternalError.into(),
                        format!(
                            "timeout: load_session exceeded {}s",
                            SESSION_TIMEOUT.as_secs()
                        ),
                    )
                })
                .and_then(|r| r)
            {
                Ok(resp) => {
                    acp_log(
                        "session.load.ok",
                        json!({ "runtime": runtime_key, "sessionId": &sid }),
                    );
                    SessionStartResult {
                        session_id: acp::SessionId::from(sid),
                        config_options: resp.config_options,
                        modes: resp.modes,
                        resumed: true,
                    }
                }
                Err(e) => {
                    acp_log(
                        "session.load.fallback",
                        json!({ "runtime": runtime_key, "error": e.to_string() }),
                    );
                    do_new_session(&conn, abs_cwd, mcp_servers).await?
                }
            }
        } else {
            do_new_session(&conn, abs_cwd, mcp_servers).await?
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

    let available_modes = start_result
        .modes
        .as_ref()
        .map(|m| {
            serde_json::to_value(m)
                .ok()
                .and_then(|v| v.get("modes").cloned())
                .unwrap_or(json!([]))
        })
        .unwrap_or(json!([]));
    let available_modes = match available_modes {
        Value::Array(a) => a,
        _ => vec![],
    };
    remember_runtime_modes(runtime_key, extract_mode_ids(&available_modes));
    let current_mode = start_result.modes.as_ref().and_then(|m| {
        serde_json::to_value(m).ok().and_then(|v| {
            v.get("current")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
    });

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
        cwd: abs_cwd.to_string(),
        delta_slot,
        available_modes,
        current_mode,
        child_pid: child.id(),
        _child: child,
        _io_task: io_handle,
        health_rx,
    })
}
