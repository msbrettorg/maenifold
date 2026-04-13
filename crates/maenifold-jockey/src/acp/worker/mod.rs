mod handlers;
mod notify;
mod permission;
mod pool;
mod types;

pub use crate::runtime_kind::RuntimeKind;
pub use notify::{set_death_event_sender, set_prewarm_event_sender};
pub use permission::respond_to_permission;
pub use types::{AcpEvent, AcpPromptResult, ConnectionDeathEvent, PrewarmEvent};

pub(crate) use permission::permission_requests;
pub(crate) use pool::{
    pool_key, register_child_pid, DeltaSlot, LiveConnection, CANCEL_HANDLES, CONN_MAP,
    DELTA_CHANNEL_CAPACITY,
};
pub(crate) use types::WorkerMsg;

use handlers::{
    handle_execute, handle_prewarm, reconnect_worker_session, reset_worker_session,
    shutdown_worker_state,
};
use serde_json::json;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};

use super::adapter::acp_log;
use agent_client_protocol::Agent as _;

static WORKER_TX: OnceLock<mpsc::UnboundedSender<WorkerMsg>> = OnceLock::new();

pub(crate) fn worker_tx() -> &'static mpsc::UnboundedSender<WorkerMsg> {
    WORKER_TX.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel::<WorkerMsg>();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("acp worker runtime");
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, run_worker(rx));
        });
        tx
    })
}

pub async fn shutdown() {
    use super::client::shutdown_terminals;
    let Some(tx) = WORKER_TX.get() else {
        shutdown_terminals().await;
        return;
    };
    let (done_tx, done_rx) = oneshot::channel();
    if tx.send(WorkerMsg::Shutdown { done_tx }).is_err() {
        shutdown_terminals().await;
        return;
    }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), done_rx).await;
}

async fn run_worker(mut rx: mpsc::UnboundedReceiver<WorkerMsg>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            WorkerMsg::Execute {
                runtime_key,
                binary,
                args,
                env,
                role_name,
                app_session_id,
                prompt,
                context,
                cwd,
                delta_tx,
                result_tx,
                auto_approve,
                mcp_servers,
                role_mode,
                role_config_options,
                resume_session_id,
            } => {
                let key = pool_key(&app_session_id, runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    handle_execute(
                        key,
                        runtime_key,
                        binary,
                        args,
                        env,
                        role_name,
                        app_session_id,
                        prompt,
                        context,
                        cwd,
                        delta_tx,
                        result_tx,
                        auto_approve,
                        mcp_servers,
                        role_mode,
                        role_config_options,
                        resume_session_id,
                    )
                    .await;
                });
            }
            WorkerMsg::Prewarm {
                runtime_key,
                binary,
                args,
                env,
                role_name,
                app_session_id,
                cwd,
                auto_approve,
                mcp_servers,
                role_mode,
                role_config_options,
                result_tx,
                resume_session_id,
            } => {
                let key = pool_key(&app_session_id, runtime_key, &role_name);
                tokio::task::spawn_local(async move {
                    handle_prewarm(
                        key,
                        runtime_key,
                        binary,
                        args,
                        env,
                        role_name,
                        app_session_id,
                        cwd,
                        auto_approve,
                        mcp_servers,
                        role_mode,
                        role_config_options,
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
                let key = pool_key(&app_session_id, runtime_key, &role_name);
                let handle = CANCEL_HANDLES.with(|m| {
                    m.borrow()
                        .get(&key)
                        .map(|h| (h.conn.clone(), h.session_id.clone()))
                });
                if let Some((conn, session_id)) = handle {
                    acp_log(
                        "cancel.sending",
                        json!({ "runtime": runtime_key, "role": role_name }),
                    );
                    tokio::task::spawn_local(async move {
                        let _ = conn
                            .cancel(agent_client_protocol::CancelNotification::new(session_id))
                            .await;
                    });
                } else {
                    acp_log(
                        "cancel.no_active_session",
                        json!({ "runtime": runtime_key, "role": role_name }),
                    );
                }
            }
            WorkerMsg::Reset {
                runtime_key,
                role_name,
                app_session_id,
                result_tx,
            } => {
                tokio::task::spawn_local(async move {
                    let result =
                        reset_worker_session(runtime_key, &role_name, &app_session_id).await;
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
                    let result =
                        reconnect_worker_session(runtime_key, &role_name, &app_session_id).await;
                    let _ = result_tx.send(result);
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
                let key = pool_key(&app_session_id, runtime_key, &role_name);
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
                let key = pool_key(&app_session_id, runtime_key, &role_name);
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
            WorkerMsg::Shutdown { done_tx } => {
                acp_log("shutdown.start", json!({}));
                shutdown_worker_state().await;
                let _ = done_tx.send(());
                break;
            }
        }
    }
}
