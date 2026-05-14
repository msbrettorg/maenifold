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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LookupResult {
    Active(Uuid),
    Terminal(Uuid),
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

    pub fn lookup(&self, server: &str, tool_use_ids: &BTreeSet<String>) -> Option<LookupResult> {
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
            return Some(LookupResult::Terminal(entry.sampling_request_id));
        }
        Some(LookupResult::Active(entry.sampling_request_id))
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

pub(crate) fn lookup(server: &str, tool_use_ids: &BTreeSet<String>) -> Option<LookupResult> {
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

#[cfg(test)]
mod tests {
    use super::{LookupResult, LoopCorrelator, TERMINAL_TTL};
    use std::collections::BTreeSet;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn concurrent_overlapping_loops_same_server_keep_distinct_tool_use_sets() {
        let correlator = LoopCorrelator::default();
        let first_request = Uuid::from_u128(0x11111111111111111111111111111111);
        let second_request = Uuid::from_u128(0x22222222222222222222222222222222);

        correlator.record(
            "same-server".to_string(),
            ids(&["tool-a", "tool-b"]),
            first_request,
            Arc::new(AtomicBool::new(false)),
        );
        correlator.record(
            "same-server".to_string(),
            ids(&["tool-c"]),
            second_request,
            Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(
            correlator.lookup("same-server", &ids(&["tool-b", "tool-a"])),
            Some(LookupResult::Active(first_request))
        );
        assert_eq!(
            correlator.lookup("same-server", &ids(&["tool-c"])),
            Some(LookupResult::Active(second_request))
        );
    }

    #[tokio::test]
    async fn cleanup_pins_active_entries_and_keeps_recent_terminal_entries() {
        let correlator = LoopCorrelator::default();
        let request_id = Uuid::from_u128(0x33333333333333333333333333333333);
        let status_handle = Arc::new(AtomicBool::new(false));
        let key = ids(&["tool-active"]);
        correlator.record(
            "server".to_string(),
            key.clone(),
            request_id,
            Arc::clone(&status_handle),
        );

        correlator.cleanup_expired().await;
        correlator.cleanup_expired().await;

        assert_eq!(
            correlator.lookup("server", &key),
            Some(LookupResult::Active(request_id))
        );

        status_handle.store(true, Ordering::SeqCst);
        assert_eq!(
            correlator.lookup("server", &key),
            Some(LookupResult::Terminal(request_id))
        );
        correlator.cleanup_expired().await;

        assert_eq!(
            correlator.lookup("server", &key),
            Some(LookupResult::Terminal(request_id))
        );
    }

    #[tokio::test]
    async fn cleanup_evicts_only_terminal_entries_older_than_ttl() {
        let correlator = LoopCorrelator::default();
        let request_id = Uuid::from_u128(0x44444444444444444444444444444444);
        let key = ids(&["tool-old"]);
        correlator.record(
            "server".to_string(),
            key.clone(),
            request_id,
            Arc::new(AtomicBool::new(true)),
        );
        assert_eq!(
            correlator.lookup("server", &key),
            Some(LookupResult::Terminal(request_id))
        );

        {
            let mut entries = correlator.entries.write().expect("test lock should open");
            let entry = entries
                .get_mut(&("server".to_string(), key.clone()))
                .expect("recorded key should exist");
            entry.terminal_at = Some(Instant::now() - TERMINAL_TTL - Duration::from_secs(1));
        }

        correlator.cleanup_expired().await;

        assert_eq!(correlator.lookup("server", &key), None);
    }

    #[test]
    fn unknown_tool_use_id_set_returns_none() {
        let correlator = LoopCorrelator::default();

        assert_eq!(correlator.lookup("server", &ids(&["never-recorded"])), None);
    }

    #[test]
    fn terminal_entry_is_distinct_from_unknown_entry() {
        let correlator = LoopCorrelator::default();
        let request_id = Uuid::from_u128(0x55555555555555555555555555555555);
        let status_handle = Arc::new(AtomicBool::new(false));
        let known_key = ids(&["known-tool"]);
        correlator.record(
            "server".to_string(),
            known_key.clone(),
            request_id,
            Arc::clone(&status_handle),
        );

        status_handle.store(true, Ordering::SeqCst);

        assert_eq!(
            correlator.lookup("server", &known_key),
            Some(LookupResult::Terminal(request_id))
        );
        assert_eq!(correlator.lookup("server", &ids(&["other-tool"])), None);
    }
}
