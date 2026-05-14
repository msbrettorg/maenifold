use std::collections::{BTreeSet, HashMap};
use std::sync::{
    Arc, OnceLock, RwLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use uuid::Uuid;

const TERMINAL_TTL: Duration = Duration::from_secs(300);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub(crate) struct CorrelatorEntry {
    pub sampling_request_id: Uuid,
    pub status_handle: Arc<AtomicBool>,
    pub terminal_at: Option<Instant>,
}

#[derive(Clone, Default)]
pub(crate) struct LoopCorrelator {
    entries: Arc<RwLock<HashMap<(String, BTreeSet<String>), CorrelatorEntry>>>,
}

impl LoopCorrelator {
    pub fn record(
        &self,
        server: String,
        tool_use_ids: BTreeSet<String>,
        request_id: Uuid,
        status_handle: Arc<AtomicBool>,
    ) {
        if tool_use_ids.is_empty() {
            return;
        }
        let entry = CorrelatorEntry {
            sampling_request_id: request_id,
            status_handle,
            terminal_at: None,
        };
        if let Ok(mut entries) = self.entries.write() {
            entries.insert((server, tool_use_ids), entry);
        }
    }

    pub fn lookup(&self, server: &str, tool_use_ids: &BTreeSet<String>) -> Option<Uuid> {
        if tool_use_ids.is_empty() {
            return None;
        }
        let key = (server.to_string(), tool_use_ids.clone());
        let mut entries = self.entries.write().ok()?;
        let entry = entries.get_mut(&key)?;
        if entry.status_handle.load(Ordering::SeqCst) {
            if entry.terminal_at.is_none() {
                entry.terminal_at = Some(Instant::now());
            }
            return None;
        }
        Some(entry.sampling_request_id)
    }

    pub async fn cleanup_expired(&self) {
        let now = Instant::now();
        if let Ok(mut entries) = self.entries.write() {
            entries.retain(|_, entry| {
                if entry.status_handle.load(Ordering::SeqCst) && entry.terminal_at.is_none() {
                    entry.terminal_at = Some(now);
                }
                entry
                    .terminal_at
                    .map(|terminal_at| now.duration_since(terminal_at) <= TERMINAL_TTL)
                    .unwrap_or(true)
            });
        }
    }
}

fn global() -> &'static LoopCorrelator {
    static GLOBAL: OnceLock<LoopCorrelator> = OnceLock::new();
    GLOBAL.get_or_init(LoopCorrelator::default)
}

pub(crate) fn record(
    server: String,
    tool_use_ids: BTreeSet<String>,
    request_id: Uuid,
    status_handle: Arc<AtomicBool>,
) {
    global().record(server, tool_use_ids, request_id, status_handle);
}

pub(crate) fn lookup(server: &str, tool_use_ids: &BTreeSet<String>) -> Option<Uuid> {
    global().lookup(server, tool_use_ids)
}

pub(crate) fn spawn_cleanup_task() {
    let correlator = global().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            interval.tick().await;
            correlator.cleanup_expired().await;
        }
    });
}
