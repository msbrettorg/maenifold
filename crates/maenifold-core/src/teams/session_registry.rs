//! Bounded registry of active Teams ACP sessions.
//!
//! Tracks one `app_session_id` per Teams `conversation_id` so the ACP worker
//! pool (`maenifold-jockey::acp::worker::pool`) naturally reuses the same
//! `LiveConnection` for every follow-up dispatch in a conversation.
//!
//! Mirrors the per-conversation session affinity from agency-cowork's
//! `monitor/prompt_queue.py` (pty_max_sessions=5, pty_idle_timeout_minutes=60).
//!
//! This module is the data structure only; it is not wired into the supervisor
//! or dispatcher in this change. See plan-T-238.61.md for the rollout plan.

#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default upper bound on concurrently tracked conversations.
/// Matches `pty_max_sessions=5` from agency-cowork prompt_queue.py L128.
pub const DEFAULT_MAX_SESSIONS: usize = 5;

/// Default idle timeout before a tracked conversation is eligible for reaping.
/// Matches `pty_idle_timeout_minutes=60` from agency-cowork prompt_queue.py L128.
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 3600;

struct SessionEntry {
    app_session_id: String,
    last_used: Instant,
}

/// Bounded LRU-evicting registry of active conversation -> `app_session_id`.
///
/// Not internally synchronized; callers wrap in `Arc<Mutex<_>>` if shared.
pub struct SessionRegistry {
    sessions: HashMap<String, SessionEntry>,
    max_sessions: usize,
    idle_timeout: Duration,
}

impl SessionRegistry {
    /// Create a new registry with the given capacity and idle timeout.
    pub fn new(max_sessions: usize, idle_timeout: Duration) -> Self {
        Self {
            sessions: HashMap::with_capacity(max_sessions.max(1)),
            max_sessions: max_sessions.max(1),
            idle_timeout,
        }
    }

    /// Return the `app_session_id` for `conversation_id`, creating
    /// `format!("teams:{conversation_id}")` on first access.
    ///
    /// Updates `last_used` on every call. Evicts the oldest idle entry if the
    /// registry is at capacity and a new entry must be inserted.
    pub fn get_or_create(&mut self, conversation_id: &str) -> &str {
        let now = Instant::now();
        if self.sessions.contains_key(conversation_id) {
            let entry = self
                .sessions
                .get_mut(conversation_id)
                .expect("contains_key just confirmed presence");
            entry.last_used = now;
            return entry.app_session_id.as_str();
        }

        if self.sessions.len() >= self.max_sessions {
            self.evict_oldest();
        }

        let app_session_id = format!("teams:{conversation_id}");
        self.sessions.insert(
            conversation_id.to_string(),
            SessionEntry {
                app_session_id,
                last_used: now,
            },
        );
        self.sessions
            .get(conversation_id)
            .map(|e| e.app_session_id.as_str())
            .expect("just inserted")
    }

    /// Update `last_used` for an existing conversation. No-op if unknown.
    pub fn touch(&mut self, conversation_id: &str) {
        if let Some(entry) = self.sessions.get_mut(conversation_id) {
            entry.last_used = Instant::now();
        }
    }

    /// Remove all entries idle longer than `idle_timeout`. Returns the number
    /// of entries reaped.
    pub fn reap_idle(&mut self) -> usize {
        let now = Instant::now();
        let timeout = self.idle_timeout;
        let before = self.sessions.len();
        self.sessions
            .retain(|_, entry| now.duration_since(entry.last_used) < timeout);
        before - self.sessions.len()
    }

    /// Number of currently tracked conversations.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .sessions
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
        {
            self.sessions.remove(&oldest_key);
        }
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_SESSIONS,
            Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_or_create_derives_app_session_id_from_conversation_id() {
        let mut reg = SessionRegistry::default();
        let id = reg.get_or_create("conv-abc").to_string();
        assert_eq!(id, "teams:conv-abc");
    }

    #[test]
    fn repeat_access_returns_same_app_session_id_and_keeps_one_entry() {
        let mut reg = SessionRegistry::default();
        let first = reg.get_or_create("conv-abc").to_string();
        let second = reg.get_or_create("conv-abc").to_string();
        assert_eq!(first, second);
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn distinct_conversations_get_distinct_session_ids() {
        let mut reg = SessionRegistry::default();
        let a = reg.get_or_create("conv-a").to_string();
        let b = reg.get_or_create("conv-b").to_string();
        assert_ne!(a, b);
        assert_eq!(reg.active_count(), 2);
    }

    #[test]
    fn evicts_oldest_when_at_capacity() {
        let mut reg = SessionRegistry::new(2, Duration::from_secs(3600));
        reg.get_or_create("conv-a");
        std::thread::sleep(Duration::from_millis(5));
        reg.get_or_create("conv-b");
        std::thread::sleep(Duration::from_millis(5));
        // Touch conv-a so conv-b becomes the oldest.
        reg.touch("conv-a");
        std::thread::sleep(Duration::from_millis(5));
        reg.get_or_create("conv-c");

        assert_eq!(reg.active_count(), 2);
        assert!(reg.sessions.contains_key("conv-a"));
        assert!(reg.sessions.contains_key("conv-c"));
        assert!(!reg.sessions.contains_key("conv-b"));
    }

    #[test]
    fn reap_idle_removes_entries_past_timeout() {
        let mut reg = SessionRegistry::new(4, Duration::from_millis(20));
        reg.get_or_create("conv-a");
        reg.get_or_create("conv-b");
        std::thread::sleep(Duration::from_millis(40));
        let reaped = reg.reap_idle();
        assert_eq!(reaped, 2);
        assert_eq!(reg.active_count(), 0);
    }

    #[test]
    fn reap_idle_keeps_recently_touched_entries() {
        let mut reg = SessionRegistry::new(4, Duration::from_millis(40));
        reg.get_or_create("conv-a");
        std::thread::sleep(Duration::from_millis(30));
        reg.touch("conv-a");
        let reaped = reg.reap_idle();
        assert_eq!(reaped, 0);
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn touch_unknown_conversation_is_noop() {
        let mut reg = SessionRegistry::default();
        reg.touch("does-not-exist");
        assert_eq!(reg.active_count(), 0);
    }
}
