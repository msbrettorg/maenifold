use agent_client_protocol::{self as acp};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::acp_log;
use super::super::runtime_state::{list_discovered_config_options, list_discovered_modes};
use super::pool::{CONN_MAP, find_session_key, register_session_index, remove_session_index};
use super::types::AcpEvent;
pub(super) async fn complete_failed_prewarm(
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    is_resume_replay: bool,
) {
    if is_resume_replay {
        if let Some(tx) = delta_tx {
            let _ = tx.send(AcpEvent::ReplayComplete).await;
        }
    }
    if let Some(tx) = result_tx {
        let _ = tx.send((vec![], vec![], String::new()));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn complete_live_resume_prewarm(
    key: &str,
    runtime_key: &str,
    role_name: &str,
    _app_session_id: &str,
    _cwd: &str,
    _mcp_servers: Vec<acp::McpServer>,
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    resume_session_id: &str,
) -> bool {
    let Some(existing_key) = find_session_key(runtime_key, role_name, resume_session_id) else {
        return false;
    };

    if existing_key != key {
        let moved = CONN_MAP.with(|m| {
            let mut map = m.borrow_mut();
            map.remove(&existing_key).map(|live| {
                let old_session_id = live.session_id.to_string();
                map.insert(key.to_string(), live);
                old_session_id
            })
        });
        if let Some(old_session_id) = moved {
            remove_session_index(runtime_key, role_name, &old_session_id);
            register_session_index(runtime_key, role_name, &old_session_id, key);
        }
    }

    let live = CONN_MAP.with(|m| {
        m.borrow()
            .get(key)
            .map(|live| (live.delta_slot.clone(), live.session_id.to_string()))
    });
    let Some((delta_slot, live_session_id)) = live else {
        return false;
    };
    if live_session_id != resume_session_id {
        return false;
    }

    if let Ok(mut slot) = delta_slot.lock() {
        *slot = delta_tx.clone();
    }

    acp_log(
        "session.live_attach.ok",
        json!({ "runtime": runtime_key, "sessionId": resume_session_id }),
    );

    if let Some(tx) = delta_tx {
        let _ = tx.send(AcpEvent::ReplayComplete).await;
    }

    if let Ok(mut slot) = delta_slot.lock() {
        *slot = None;
    }

    if let Some(tx) = result_tx {
        let _ = tx.send((
            list_discovered_config_options(runtime_key),
            list_discovered_modes(runtime_key),
            resume_session_id.to_string(),
        ));
    }
    true
}

pub(super) async fn complete_already_live_prewarm(
    key: &str,
    runtime_key: &str,
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    is_resume_replay: bool,
) {
    let session_id = CONN_MAP.with(|m| {
        m.borrow()
            .get(key)
            .map(|c| c.session_id.to_string())
            .unwrap_or_default()
    });

    if is_resume_replay {
        if let Some(tx) = delta_tx {
            let _ = tx.send(AcpEvent::ReplayComplete).await;
        }
    }

    if let Some(tx) = result_tx {
        let _ = tx.send((
            list_discovered_config_options(runtime_key),
            list_discovered_modes(runtime_key),
            session_id,
        ));
    }
}
