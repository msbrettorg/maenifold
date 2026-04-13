use agent_client_protocol as acp;
use dashmap::DashSet;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

use super::types::AcpEvent;

pub(crate) const DELTA_CHANNEL_CAPACITY: usize = 512;
pub(crate) type DeltaSlot = Arc<Mutex<Option<mpsc::Sender<AcpEvent>>>>;

// ── Pool key ──────────────────────────────────────────────────────────────────

pub(crate) fn pool_key(app_session_id: &str, runtime_key: &str, role_name: &str) -> String {
    format!("{app_session_id}:{runtime_key}:{role_name}")
}

// ── LiveConnection ────────────────────────────────────────────────────────────

pub(crate) struct LiveConnection {
    pub(crate) instance_id: u64,
    pub(crate) conn: Rc<acp::ClientSideConnection>,
    pub(crate) session_id: acp::SessionId,
    pub(crate) cwd: String,
    pub(crate) delta_slot: DeltaSlot,
    #[allow(dead_code)]
    pub(crate) available_modes: Vec<Value>,
    #[allow(dead_code)]
    pub(crate) current_mode: Option<String>,
    pub(crate) child_pid: Option<u32>,
    pub(crate) _child: tokio::process::Child,
    pub(crate) _io_task: tokio::task::JoinHandle<()>,
    pub(crate) health_rx: tokio::sync::watch::Receiver<bool>,
}

impl Drop for LiveConnection {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid {
            unsafe {
                let pgid = -(pid as i32);
                let _ = libc::kill(pgid, libc::SIGTERM);
                let _ = libc::kill(pgid, libc::SIGKILL);
                let _ = libc::kill(pid as i32, libc::SIGTERM);
                let _ = libc::kill(pid as i32, libc::SIGKILL);
            }
            unregister_child_pid(pid);
        }
    }
}

// SAFETY: LiveConnection is only ever created and accessed on the single-threaded
// worker LocalSet (see `run_worker`). No actual cross-thread access occurs.
// The Rc<ClientSideConnection> is the only non-Send field; it is safe because:
// 1. cold_start() runs on the worker LocalSet and produces the Rc.
// 2. handle_execute/handle_prewarm run on the same LocalSet.
// 3. The connection map is thread-local to that worker thread.
unsafe impl Send for LiveConnection {}
unsafe impl Sync for LiveConnection {}

// ── CancelHandle: lightweight per-prompt cancel token ────────────────────────

/// A lightweight handle for sending ACP cancel without holding the conn mutex.
/// Stored in a thread-local map keyed by pool_key, only accessed on the worker
/// thread's LocalSet — so `Rc` is fine.
pub(crate) struct CancelHandle {
    pub(crate) conn: Rc<acp::ClientSideConnection>,
    pub(crate) session_id: acp::SessionId,
}

// ── Thread-local connection map (worker-thread only) ──────────────────────────
//
// Each entry is a `LiveConnection` directly — no Arc<SlotHandle> wrapping.
// All access happens on the single worker thread's LocalSet, so thread-safety
// is guaranteed by the single-threaded execution model.

thread_local! {
    /// The primary connection store: pool_key → LiveConnection.
    pub(crate) static CONN_MAP: RefCell<std::collections::HashMap<String, LiveConnection>> =
        RefCell::new(std::collections::HashMap::new());

    /// Per-prompt cancel handles: pool_key → CancelHandle.
    pub(crate) static CANCEL_HANDLES: RefCell<std::collections::HashMap<String, CancelHandle>> =
        RefCell::new(std::collections::HashMap::new());

    /// Per-slot prompt serialization: pool_key → async mutex.
    pub(crate) static PROMPT_LOCKS: RefCell<HashMap<String, Arc<AsyncMutex<()>>>> =
        RefCell::new(HashMap::new());
}

// ── Global state: child PIDs (still shared for cross-thread shutdown) ─────────

static CHILD_PIDS: OnceLock<DashSet<u32>> = OnceLock::new();
pub(crate) fn child_pids() -> &'static DashSet<u32> {
    CHILD_PIDS.get_or_init(DashSet::new)
}

pub(crate) fn register_child_pid(pid: u32) {
    child_pids().insert(pid);
}

pub(crate) fn unregister_child_pid(pid: u32) {
    child_pids().remove(&pid);
}
