use agent_client_protocol as acp;
use dashmap::DashSet;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

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
    pub(crate) resumed: bool,
    pub(crate) cwd: String,
    pub(crate) delta_slot: DeltaSlot,
    #[allow(dead_code)]
    pub(crate) available_modes: Vec<Value>,
    #[allow(dead_code)]
    pub(crate) current_mode: Option<String>,
    pub(crate) child_pid: Option<u32>,
    pub(crate) _io_task: tokio::task::JoinHandle<()>,
    pub(crate) health_rx: tokio::sync::watch::Receiver<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedSessionReap {
    pub acp_session: bool,
    pub killed_process_group: bool,
    pub clean_cancel_completed: bool,
}

impl Drop for LiveConnection {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid {
            terminate_child_process(pid);
            unregister_child_pid(pid);
            // T-328.16 v2 — purge cross-thread pool→pid index for any entry
            // whose pid matches (handles PID reuse safely; O(N) over the
            // small live-connection set).
            if let Some(map) = POOL_PIDS.get() {
                if let Ok(mut guard) = map.lock() {
                    guard.retain(|_, p| *p != pid);
                }
            }
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
    pub(crate) cancelled: Arc<AtomicBool>,
}

impl CancelHandle {
    pub(crate) fn mark_cancelled(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
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

    /// Remote ACP session id → current pool key for live connections.
    pub(crate) static SESSION_INDEX: RefCell<HashMap<String, String>> =
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

// ── T-328.16 v2: cross-thread pool_key → child pid index ──────────────────────
//
// The thread_local CONN_MAP lives on the worker LocalSet thread and cannot be
// inspected from the executor runtime. To kill a hung child from the budget
// timeout arm WITHOUT routing through the worker actor mpsc (which is blocked
// behind in-flight `session.prompt()` futures), we mirror just the child pid
// in a global Mutex<HashMap> here.

static POOL_PIDS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
fn pool_pids() -> &'static Mutex<HashMap<String, u32>> {
    POOL_PIDS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn register_pool_pid(pool_key: &str, pid: u32) {
    if let Ok(mut guard) = pool_pids().lock() {
        guard.insert(pool_key.to_string(), pid);
    }
}

#[cfg(unix)]
pub(crate) fn terminate_child_process(pid: u32) {
    unsafe {
        let pgid = -(pid as i32);
        let _ = libc::kill(pgid, libc::SIGTERM);
        let _ = libc::kill(pgid, libc::SIGKILL);
        let _ = libc::kill(pid as i32, libc::SIGTERM);
        let _ = libc::kill(pid as i32, libc::SIGKILL);
    }
}

#[cfg(windows)]
pub(crate) fn terminate_child_process(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status();
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn terminate_child_process(_pid: u32) {}

/// Synchronously terminate the ACP child process for the given pool slot.
/// Safe to call from any thread — performs only a Mutex lock and a platform
/// process termination call, no channel hop and no await. Returns true if a pid
/// was found and termination was attempted.
pub fn kill_pool_child_direct(pool_key: &str) -> bool {
    let pid_opt = pool_pids()
        .lock()
        .ok()
        .and_then(|g| g.get(pool_key).copied());
    if let Some(pid) = pid_opt {
        terminate_child_process(pid);
        true
    } else {
        false
    }
}

pub(crate) fn session_index_key(runtime_key: &str, role_name: &str, session_id: &str) -> String {
    format!("{runtime_key}:{role_name}:{session_id}")
}

pub(crate) fn register_session_index(
    runtime_key: &str,
    role_name: &str,
    session_id: &str,
    key: &str,
) {
    SESSION_INDEX.with(|m| {
        m.borrow_mut().insert(
            session_index_key(runtime_key, role_name, session_id),
            key.to_string(),
        );
    });
}

pub(crate) fn remove_session_index(runtime_key: &str, role_name: &str, session_id: &str) {
    SESSION_INDEX.with(|m| {
        m.borrow_mut()
            .remove(&session_index_key(runtime_key, role_name, session_id));
    });
}

pub(crate) fn remove_session_index_for_pool_key(key: &str) {
    SESSION_INDEX.with(|m| {
        m.borrow_mut().retain(|_, value| value != key);
    });
}

pub(crate) fn find_session_key(
    runtime_key: &str,
    role_name: &str,
    session_id: &str,
) -> Option<String> {
    SESSION_INDEX.with(|m| {
        m.borrow()
            .get(&session_index_key(runtime_key, role_name, session_id))
            .cloned()
    })
}
