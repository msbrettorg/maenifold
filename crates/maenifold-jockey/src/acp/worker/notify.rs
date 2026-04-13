use std::sync::OnceLock;
use tokio::sync::mpsc;

use super::types::{ConnectionDeathEvent, PrewarmEvent, PrewarmStatus};

static DEATH_TX: OnceLock<mpsc::UnboundedSender<ConnectionDeathEvent>> = OnceLock::new();
static PREWARM_TX: OnceLock<mpsc::UnboundedSender<PrewarmEvent>> = OnceLock::new();

pub fn set_death_event_sender(tx: mpsc::UnboundedSender<ConnectionDeathEvent>) {
    let _ = DEATH_TX.set(tx);
}

pub fn set_prewarm_event_sender(tx: mpsc::UnboundedSender<PrewarmEvent>) {
    let _ = PREWARM_TX.set(tx);
}

pub(crate) fn notify_prewarm(
    runtime_key: &str,
    role_name: &str,
    app_session_id: &str,
    status: PrewarmStatus,
) {
    if let Some(tx) = PREWARM_TX.get() {
        let _ = tx.send(PrewarmEvent {
            runtime_key: runtime_key.to_string(),
            role_name: role_name.to_string(),
            app_session_id: app_session_id.to_string(),
            status,
        });
    }
}

pub(crate) fn notify_connection_death(event: ConnectionDeathEvent) {
    if let Some(tx) = DEATH_TX.get() {
        let _ = tx.send(event);
    }
}
