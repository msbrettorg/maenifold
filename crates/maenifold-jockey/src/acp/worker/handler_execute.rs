use agent_client_protocol::{self as acp, Agent as _};
use serde_json::json;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, resolve_cwd};
use super::handler_config::{
    apply_cold_start_config, build_prompt_blocks, spawn_connection_health_watch,
};
use super::handlers::{ensure_connection, evict_if_cwd_changed};
use super::pool::{CANCEL_HANDLES, CONN_MAP, CancelHandle, PROMPT_LOCKS};
use super::resume_execute::{
    generic_error, normalize_prompt_result_after_cancel, prompt_error, resume_start_error,
};
use super::types::{AcpEvent, AcpExecuteError, AcpExecuteSuccess, ConnectionDeathEvent};

const PROMPT_LIVENESS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_execute(
    key: String,
    runtime_key: &str,
    binary: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    client_capabilities: acp::ClientCapabilities,
    role_name: String,
    app_session_id: String,
    prompt: String,
    context: Vec<(String, String)>,
    prompt_blocks: Option<Vec<acp::ContentBlock>>,
    cwd: String,
    delta_tx: mpsc::Sender<AcpEvent>,
    result_tx: oneshot::Sender<Result<AcpExecuteSuccess, AcpExecuteError>>,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    role_mode: Option<String>,
    role_config_options: Vec<(String, String)>,
    resume_session_id: Option<String>,
    session_id_tx: Option<oneshot::Sender<String>>,
) {
    let prompt_lock = PROMPT_LOCKS.with(|m| {
        let mut map = m.borrow_mut();
        map.entry(key.clone())
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    });
    let prompt_guard = prompt_lock.lock().await;
    let requested_resume = resume_session_id.is_some();
    let resolved = resolve_cwd(&cwd);

    evict_if_cwd_changed(&key, &resolved, runtime_key, &role_name);

    let conn_exists = CONN_MAP.with(|m| m.borrow().contains_key(&key));
    if !conn_exists {
        let _ = delta_tx.try_send(AcpEvent::StatusUpdate {
            text: "Connecting to agent runtime...".to_string(),
        });
    }

    let is_cold = match ensure_connection(
        &key,
        runtime_key,
        &binary,
        &args,
        &env,
        client_capabilities,
        &resolved,
        &role_name,
        &app_session_id,
        auto_approve,
        mcp_servers,
        resume_session_id,
    )
    .await
    {
        Ok(cold) => cold,
        Err(e) => {
            let _ = result_tx.send(Err(resume_start_error(e, requested_resume)));
            return;
        }
    };

    let (session_id, resumed) = CONN_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .map(|c| (c.session_id.clone(), c.resumed))
            .unwrap_or_else(|| (acp::SessionId::from(String::new()), false))
    });
    let delta_slot = CONN_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .map(|c| c.delta_slot.clone())
            .unwrap_or_else(|| Arc::new(Mutex::new(None)))
    });

    if let Ok(mut slot_guard) = delta_slot.lock() {
        *slot_guard = Some(delta_tx.clone());
    }

    let conn_rc = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.conn.clone()));
    let health_rx = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.health_rx.clone()));
    let instance_id = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.instance_id));

    let (Some(conn_rc), Some(mut health_rx), Some(instance_id)) = (conn_rc, health_rx, instance_id)
    else {
        let message = "connection disappeared after cold start".to_string();
        let _ = result_tx.send(Err(prompt_error(message, requested_resume, resumed)));
        return;
    };
    let session_id_text = session_id.to_string();
    if let (false, Some(tx)) = (session_id_text.is_empty(), session_id_tx) {
        let _ = tx.send(session_id_text);
    }

    if is_cold {
        let health_rx_clone = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.health_rx.clone()));
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_mode,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(generic_error(e, requested_resume, resumed)));
            return;
        }

        let death = ConnectionDeathEvent {
            runtime_key: runtime_key.to_string(),
            role_name: role_name.clone(),
            app_session_id: app_session_id.clone(),
        };
        if let Some(hrx) = health_rx_clone {
            spawn_connection_health_watch(key.clone(), instance_id, death, hrx);
        }
    } else {
        // T-170.8: Re-apply config options on warm connections so that
        // model/thoughtLevel overrides from each pipeline node reach the agent.
        // Previously only modes were re-emitted here, silently dropping
        // per-node model and thoughtLevel overrides.
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_mode,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(generic_error(e, requested_resume, resumed)));
            return;
        }
    }

    let blocks = prompt_blocks.unwrap_or_else(|| build_prompt_blocks(prompt, &context));

    // Install cancel handle so the Cancel message handler can send ACP cancel
    // without touching CONN_MAP.
    let cancelled = Arc::new(AtomicBool::new(false));
    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().insert(
            key.clone(),
            CancelHandle {
                conn: conn_rc.clone(),
                session_id: session_id.clone(),
                cancelled: cancelled.clone(),
            },
        );
    });

    let prompt_started = Instant::now();
    let prompt_fut = conn_rc.prompt(acp::PromptRequest::new(session_id.clone(), blocks));
    tokio::pin!(prompt_fut);

    let prompt_result: Result<Result<acp::PromptResponse, acp::Error>, String> = loop {
        tokio::select! {
            res = &mut prompt_fut => {
                break Ok(res);
            }
            changed = health_rx.changed() => {
                if changed.is_err() || !*health_rx.borrow() {
                    break Err(format!(
                        "Agent '{runtime_key}' exited: prompt was in progress when the process stopped"
                    ));
                }
            }
            _ = tokio::time::sleep(PROMPT_LIVENESS_INTERVAL) => {
                if !*health_rx.borrow() {
                    break Err(format!("Agent '{runtime_key}' exited: process is no longer alive"));
                }
            }
        }
    };

    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().remove(&key);
    });
    drop(prompt_guard);

    if let Ok(mut slot_guard) = delta_slot.lock() {
        *slot_guard = None;
    }

    let was_cancelled = cancelled.load(Ordering::SeqCst);
    match prompt_result {
        Ok(Ok(resp)) => {
            acp_log(
                "stage.ok",
                json!({
                    "runtime": runtime_key,
                    "stage": "session/prompt",
                    "latencyMs": prompt_started.elapsed().as_millis(),
                    "stopReason": format!("{:?}", resp.stop_reason)
                }),
            );
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Ok(AcpExecuteSuccess {
                    output: String::new(),
                    session_id: session_id.to_string(),
                    resumed,
                    stop_reason: resp.stop_reason,
                }),
                was_cancelled,
                &session_id,
                resumed,
            ));
        }
        Ok(Err(e)) => {
            acp_log(
                "pool.invalidate",
                json!({ "runtime": runtime_key, "error": e.to_string() }),
            );
            // Invalidate connection only if still the same instance.
            let is_same = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.instance_id == instance_id)
                    .unwrap_or(false)
            });
            if is_same {
                CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            }
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Err(prompt_error(e.to_string(), requested_resume, resumed)),
                was_cancelled,
                &session_id,
                resumed,
            ));
        }
        Err(reason) => {
            acp_log(
                "pool.prompt.process_died",
                json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "elapsedSec": prompt_started.elapsed().as_secs(),
                    "reason": reason
                }),
            );
            let _ = conn_rc
                .cancel(acp::CancelNotification::new(session_id.clone()))
                .await;
            let is_same = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.instance_id == instance_id)
                    .unwrap_or(false)
            });
            if is_same {
                CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            }
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Err(prompt_error(reason, requested_resume, resumed)),
                was_cancelled,
                &session_id,
                resumed,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    #[derive(Clone, Default)]
    struct CapturingAgent {
        prompts: Arc<Mutex<Vec<acp::PromptRequest>>>,
    }

    #[async_trait::async_trait(?Send)]
    impl acp::Agent for CapturingAgent {
        async fn initialize(
            &self,
            request: acp::InitializeRequest,
        ) -> acp::Result<acp::InitializeResponse> {
            Ok(acp::InitializeResponse::new(request.protocol_version))
        }

        async fn authenticate(
            &self,
            _request: acp::AuthenticateRequest,
        ) -> acp::Result<acp::AuthenticateResponse> {
            Ok(acp::AuthenticateResponse::default())
        }

        async fn new_session(
            &self,
            _request: acp::NewSessionRequest,
        ) -> acp::Result<acp::NewSessionResponse> {
            Ok(acp::NewSessionResponse::new(acp::SessionId::new(
                "session-1",
            )))
        }

        async fn prompt(&self, request: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
            self.prompts.lock().unwrap().push(request);
            Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
        }

        async fn cancel(&self, _request: acp::CancelNotification) -> acp::Result<()> {
            Ok(())
        }
    }

    fn insert_mock_connection(
        key: &str,
        agent: CapturingAgent,
    ) -> tokio::sync::watch::Sender<bool> {
        let (client_stream, agent_stream) = tokio::io::duplex(8192);
        let (client_read, client_write) = tokio::io::split(client_stream);
        let (agent_read, agent_write) = tokio::io::split(agent_stream);

        let delta_slot = Arc::new(Mutex::new(None));
        let (client_conn, client_io_task) = acp::ClientSideConnection::new(
            super::super::super::client::JockeyUiClient {
                delta_slot,
                auto_approve: true,
                workspace_root: PathBuf::from("/"),
            },
            client_write.compat_write(),
            client_read.compat(),
            |fut| {
                tokio::task::spawn_local(fut);
            },
        );

        let (_agent_conn, agent_io_task) = acp::AgentSideConnection::new(
            agent,
            agent_write.compat_write(),
            agent_read.compat(),
            |fut| {
                tokio::task::spawn_local(fut);
            },
        );
        tokio::task::spawn_local(agent_io_task);

        let (health_tx, health_rx) = tokio::sync::watch::channel(true);
        let child = tokio::process::Command::new("true")
            .spawn()
            .expect("spawn inert child process for LiveConnection test fixture");

        CONN_MAP.with(|map| {
            map.borrow_mut().insert(
                key.to_string(),
                super::super::pool::LiveConnection {
                    instance_id: 1,
                    conn: std::rc::Rc::new(client_conn),
                    session_id: acp::SessionId::new("session-1"),
                    resumed: false,
                    cwd: "/".to_string(),
                    delta_slot: Arc::new(Mutex::new(None)),
                    available_modes: Vec::new(),
                    current_mode: None,
                    child_pid: None,
                    _child: child,
                    _io_task: tokio::task::spawn_local(async move {
                        let _ = client_io_task.await;
                    }),
                    health_rx,
                },
            );
        });
        health_tx
    }

    #[tokio::test(flavor = "current_thread")]
    async fn handler_execute_forwards_multipart_prompt_blocks_to_upstream_agent() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let key = super::super::pool::pool_key("app-session", "mock-runtime", "mock-role");
                let agent = CapturingAgent::default();
                let prompts = agent.prompts.clone();
                let _health_tx = insert_mock_connection(&key, agent);

                let image =
                    acp::ContentBlock::Image(acp::ImageContent::new("iVBORw0KGgo=", "image/png"));
                let audio =
                    acp::ContentBlock::Audio(acp::AudioContent::new("UklGRg==", "audio/wav"));
                let text = acp::ContentBlock::Text(acp::TextContent::new("keep this text block"));
                let prompt_blocks = vec![image.clone(), audio.clone(), text.clone()];

                let (delta_tx, _delta_rx) = mpsc::channel(16);
                let (result_tx, result_rx) = oneshot::channel();

                handle_execute(
                    key.clone(),
                    "mock-runtime",
                    "unused-binary".to_string(),
                    Vec::new(),
                    Vec::new(),
                    acp::ClientCapabilities::default(),
                    "mock-role".to_string(),
                    "app-session".to_string(),
                    "fallback text should not replace prompt_blocks".to_string(),
                    Vec::new(),
                    Some(prompt_blocks.clone()),
                    "/".to_string(),
                    delta_tx,
                    result_tx,
                    true,
                    Vec::new(),
                    None,
                    Vec::new(),
                    None,
                    None,
                )
                .await;

                let result = result_rx.await.expect("worker result sent");
                assert!(result.is_ok(), "prompt should complete: {result:?}");

                let captured = prompts.lock().unwrap().clone();
                assert_eq!(captured.len(), 1);
                assert_eq!(captured[0].prompt, prompt_blocks);

                CONN_MAP.with(|map| {
                    map.borrow_mut().remove(&key);
                });
            })
            .await;
    }
}
