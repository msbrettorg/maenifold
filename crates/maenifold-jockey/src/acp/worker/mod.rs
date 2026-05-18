mod dispatch;
mod handler_cancel_tracked;
mod handler_config;
mod handler_execute;
mod handler_prewarm;
mod handlers;
mod notify;
mod permission;
mod pool;
mod prewarm_complete;
mod resume_execute;
mod types;

pub use notify::{set_death_event_sender, set_prewarm_event_sender};
pub use permission::respond_to_permission;
pub use types::{
    AcpEvent, AcpExecuteError, AcpExecuteResult, AcpExecuteSuccess, AcpPromptResult,
    ConnectionDeathEvent, PrewarmEvent, WorkerMsg,
};

pub(crate) use permission::permission_requests;
pub use pool::TrackedSessionReap;
pub use pool::kill_pool_child_direct;
pub(crate) use pool::{DELTA_CHANNEL_CAPACITY, DeltaSlot, LiveConnection, register_child_pid};
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};

use dispatch::run_worker;
static WORKER_TX: OnceLock<mpsc::UnboundedSender<WorkerMsg>> = OnceLock::new();

pub fn worker_tx() -> &'static mpsc::UnboundedSender<WorkerMsg> {
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
