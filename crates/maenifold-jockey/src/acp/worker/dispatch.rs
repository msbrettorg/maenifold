use agent_client_protocol::Agent as _;
use serde_json::json;
use tokio::sync::mpsc;

use super::super::adapter::acp_log;
use super::handlers;
use super::pool::{CONN_MAP, pool_key};
use super::types::WorkerMsg;

pub(super) async fn run_worker(mut rx: mpsc::UnboundedReceiver<WorkerMsg>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            WorkerMsg::Execute {
                runtime_key,
                binary,
                args,
                env,
                client_capabilities,
                role_name,
                app_session_id,
                prompt,
                context,
                prompt_blocks,
                cwd,
                delta_tx,
                result_tx,
                auto_approve,
                mcp_servers,
                role_mode,
                role_config_options,
                resume_session_id,
                session_id_tx,
            } => {
                let key = pool_key(&app_session_id, &runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    handlers::handle_execute(
                        key,
                        &runtime_key,
                        binary,
                        args,
                        env,
                        client_capabilities,
                        role_name,
                        app_session_id,
                        prompt,
                        context,
                        prompt_blocks,
                        cwd,
                        delta_tx,
                        result_tx,
                        auto_approve,
                        mcp_servers,
                        role_mode,
                        role_config_options,
                        resume_session_id,
                        session_id_tx,
                    )
                    .await;
                });
            }
            WorkerMsg::Prewarm {
                runtime_key,
                binary,
                args,
                env,
                client_capabilities,
                role_name,
                app_session_id,
                cwd,
                auto_approve,
                mcp_servers,
                role_mode,
                role_config_options,
                delta_tx,
                result_tx,
                resume_session_id,
            } => {
                let key = pool_key(&app_session_id, &runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    handlers::handle_prewarm(
                        key,
                        &runtime_key,
                        binary,
                        args,
                        env,
                        client_capabilities,
                        role_name,
                        app_session_id,
                        cwd,
                        auto_approve,
                        mcp_servers,
                        role_mode,
                        role_config_options,
                        delta_tx,
                        result_tx,
                        resume_session_id,
                    )
                    .await;
                });
            }
            WorkerMsg::Cancel {
                runtime_key,
                role_name,
                app_session_id,
            } => {
                handlers::handle_cancel(runtime_key, role_name, app_session_id);
            }
            WorkerMsg::Reset {
                runtime_key,
                role_name,
                app_session_id,
                result_tx,
            } => {
                tokio::task::spawn_local(async move {
                    let result =
                        handlers::reset_worker_session(&runtime_key, &role_name, &app_session_id)
                            .await;
                    let _ = result_tx.send(result);
                });
            }
            WorkerMsg::ResetBySession {
                runtime_key,
                role_name,
                session_id,
                result_tx,
            } => {
                tokio::task::spawn_local(async move {
                    let result = handlers::reset_worker_session_by_session_id(
                        &runtime_key,
                        &role_name,
                        &session_id,
                    )
                    .await;
                    let _ = result_tx.send(result);
                });
            }
            WorkerMsg::Reconnect {
                runtime_key,
                role_name,
                app_session_id,
                result_tx,
            } => {
                tokio::task::spawn_local(async move {
                    let result = handlers::reconnect_worker_session(
                        &runtime_key,
                        &role_name,
                        &app_session_id,
                    )
                    .await;
                    let _ = result_tx.send(result);
                });
            }
            WorkerMsg::GetResumeState {
                runtime_key,
                role_name,
                app_session_id,
                result_tx,
            } => {
                let key = pool_key(&app_session_id, &runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    let resumed = CONN_MAP.with(|m| {
                        m.borrow()
                            .get(&key)
                            .map(|live| live.resumed)
                            .unwrap_or(false)
                    });
                    let _ = result_tx.send(resumed);
                });
            }
            WorkerMsg::SetMode {
                runtime_key,
                role_name,
                app_session_id,
                mode_id,
                result_tx,
            } => {
                use agent_client_protocol as acp;
                let key = pool_key(&app_session_id, &runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    let result = CONN_MAP.with(|m| {
                        let map = m.borrow();
                        if let Some(live) = map.get(&key) {
                            Some((live.conn.clone(), live.session_id.clone()))
                        } else {
                            None
                        }
                    });
                    let result = if let Some((conn, session_id)) = result {
                        conn.set_session_mode(acp::SetSessionModeRequest::new(
                            session_id,
                            acp::SessionModeId::from(mode_id),
                        ))
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                    } else {
                        Err("no active session".to_string())
                    };
                    let _ = result_tx.send(result);
                });
            }
            WorkerMsg::SetConfigOption {
                runtime_key,
                role_name,
                app_session_id,
                config_id,
                value,
                result_tx,
            } => {
                use agent_client_protocol as acp;
                let key = pool_key(&app_session_id, &runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    let result = CONN_MAP.with(|m| {
                        let map = m.borrow();
                        if let Some(live) = map.get(&key) {
                            Some((live.conn.clone(), live.session_id.clone()))
                        } else {
                            None
                        }
                    });
                    let result = if let Some((conn, session_id)) = result {
                        conn.set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                            session_id,
                            acp::SessionConfigId::from(config_id),
                            acp::SessionConfigValueId::from(value),
                        ))
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                    } else {
                        Err("no active session".to_string())
                    };
                    let _ = result_tx.send(result);
                });
            }
            WorkerMsg::CancelTrackedSession {
                session_id,
                result_tx,
            } => {
                handlers::handle_cancel_tracked_session(session_id, result_tx);
            }
            WorkerMsg::ListLiveSessions { result_tx } => {
                let _ = result_tx.send(handlers::live_session_ids());
            }
            WorkerMsg::Shutdown { done_tx } => {
                acp_log("shutdown.start", json!({}));
                handlers::shutdown_worker_state().await;
                let _ = done_tx.send(());
                break;
            }
        }
    }
}
