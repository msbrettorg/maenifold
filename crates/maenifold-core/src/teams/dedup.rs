//! Bounded message-ID deduplication buffer.
//!
//! Trouter redelivers messages on `trouter.reconnect` and re-registration, so the
//! same `message_id` can flow through `process_messages` more than once. This
//! buffer rejects redelivered IDs before dispatch.
//!
//! Deliberately separate from per-conversation cooldown: dedup is a *correctness*
//! gate (drop transport artifacts), cooldown is a *rate limit* (throttle distinct
//! prompts). They use different state and different semantics.
//!
//! See agency-cowork `service.py::_route_message` (L580-599) for reference behavior.

use std::collections::{HashSet, VecDeque};

/// Default capacity matches agency-cowork's `_processed_ids` set.
pub const DEFAULT_DEDUP_CAPACITY: usize = 2000;

/// Fixed-capacity FIFO set of message IDs.
///
/// Backed by a `VecDeque` (insertion order) and `HashSet` (O(1) membership).
/// On overflow, the oldest entry is evicted from both.
#[derive(Debug)]
pub struct MessageDedup {
    order: VecDeque<String>,
    seen: HashSet<String>,
    capacity: usize,
}

impl MessageDedup {
    /// Create a new dedup buffer with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            order: VecDeque::with_capacity(capacity),
            seen: HashSet::with_capacity(capacity),
            capacity,
        }
    }

    /// Check membership and record the id atomically.
    ///
    /// Returns `true` if the id was newly recorded (caller should proceed),
    /// `false` if it was already present (caller should skip as a duplicate).
    ///
    /// Empty IDs are not deduped — they always return `true` and are not stored,
    /// to avoid collapsing every malformed event into a single bucket.
    pub fn check_and_record(&mut self, id: &str) -> bool {
        if id.is_empty() {
            return true;
        }
        if self.seen.contains(id) {
            return false;
        }
        if self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.seen.remove(&oldest);
            }
        }
        self.order.push_back(id.to_string());
        self.seen.insert(id.to_string());
        true
    }
}

impl Default for MessageDedup {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_DEDUP_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_id_is_rejected() {
        let mut dedup = MessageDedup::with_capacity(8);
        assert!(dedup.check_and_record("msg-1"));
        assert!(!dedup.check_and_record("msg-1"));
    }

    #[test]
    fn distinct_ids_both_pass() {
        let mut dedup = MessageDedup::with_capacity(8);
        assert!(dedup.check_and_record("msg-1"));
        assert!(dedup.check_and_record("msg-2"));
    }

    #[test]
    fn capacity_overflow_evicts_oldest() {
        let mut dedup = MessageDedup::with_capacity(3);
        assert!(dedup.check_and_record("a"));
        assert!(dedup.check_and_record("b"));
        assert!(dedup.check_and_record("c"));
        // "d" pushes out "a" (oldest)
        assert!(dedup.check_and_record("d"));
        // "a" is gone, so it is accepted again
        assert!(dedup.check_and_record("a"));
        // "c" is still in the buffer (was [b,c,d] then [c,d,a])
        assert!(!dedup.check_and_record("c"));
    }

    #[test]
    fn empty_id_is_not_deduped() {
        let mut dedup = MessageDedup::with_capacity(8);
        assert!(dedup.check_and_record(""));
        assert!(dedup.check_and_record(""));
        // Real ids are unaffected
        assert!(dedup.check_and_record("real"));
        assert!(!dedup.check_and_record("real"));
    }
}
