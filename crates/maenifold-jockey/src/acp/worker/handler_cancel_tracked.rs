use std::time::Duration;

use agent_client_protocol::schema::{self as acp};
use tokio::sync::oneshot;

use super::pool::{
    CANCEL_HANDLES, CONN_MAP, TrackedSessionReap, remove_session_index_for_pool_key,
};

const CLEAN_CANCEL_TIMEOUT: Duration = Duration::from_secs(3);

#[cfg(test)]
static TEST_CLEAN_CANCEL_HANGS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(all(test, unix))]
pub(super) fn set_test_clean_cancel_hangs(hangs: bool) {
    TEST_CLEAN_CANCEL_HANGS.store(hangs, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
fn test_clean_cancel_hangs() -> bool {
    TEST_CLEAN_CANCEL_HANGS.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(not(test))]
fn test_clean_cancel_hangs() -> bool {
    false
}

pub(crate) async fn cancel_tracked_session(session_id: &str) -> Result<TrackedSessionReap, String> {
    let target = CONN_MAP.with(|m| {
        m.borrow()
            .iter()
            .find(|(_, live)| live.session_id.to_string() == session_id)
            .map(|(key, live)| (key.clone(), live.child_pid.is_some()))
    });
    let Some((key, had_process_group)) = target else {
        return Ok(TrackedSessionReap {
            acp_session: false,
            killed_process_group: false,
            clean_cancel_completed: false,
        });
    };

    let cancel = CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
    let mut clean_cancel_completed = false;
    if let Some(h) = cancel {
        if test_clean_cancel_hangs() {
            clean_cancel_completed =
                tokio::time::timeout(CLEAN_CANCEL_TIMEOUT, std::future::pending::<()>())
                    .await
                    .is_ok();
        } else {
            clean_cancel_completed = tokio::time::timeout(CLEAN_CANCEL_TIMEOUT, async {
                h.conn
                    .send_notification(acp::CancelNotification::new(h.session_id))
                    .is_ok()
            })
            .await
            .unwrap_or(false);
        }
    }

    if let Some(live) = CONN_MAP.with(|m| m.borrow_mut().remove(&key)) {
        remove_session_index_for_pool_key(&key);
        drop(live);
    }
    Ok(TrackedSessionReap {
        acp_session: true,
        killed_process_group: had_process_group,
        clean_cancel_completed,
    })
}

pub(crate) fn handle_cancel_tracked_session(
    session_id: String,
    result_tx: oneshot::Sender<Result<TrackedSessionReap, String>>,
) {
    tokio::task::spawn_local(async move {
        let result = cancel_tracked_session(&session_id).await;
        let _ = result_tx.send(result);
    });
}

#[cfg(test)]
#[path = "handler_cancel_tracked_tests.rs"]
mod tests;
