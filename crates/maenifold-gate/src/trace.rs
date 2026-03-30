use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub timestamp: String,
    pub direction: Direction,
    pub method: String,
    pub id: Option<serde_json::Value>,
    pub body: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Direction {
    Incoming,
    Outgoing,
}

#[derive(Clone)]
pub struct TraceBuffer {
    entries: Arc<Mutex<VecDeque<TraceEntry>>>,
    capacity: usize,
}

impl TraceBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&self, entry: TraceEntry) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };

        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    pub fn snapshot(&self) -> Vec<TraceEntry> {
        self.entries
            .lock()
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, TraceBuffer, TraceEntry};

    #[test]
    fn evicts_oldest_entry_when_capacity_is_reached() {
        let buffer = TraceBuffer::new(2);
        for idx in 0..3 {
            buffer.push(TraceEntry {
                timestamp: format!("00:00:00.00{idx}"),
                direction: Direction::Incoming,
                method: format!("method-{idx}"),
                id: None,
                body: "{}".to_string(),
                session_id: None,
            });
        }

        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].method, "method-1");
        assert_eq!(snapshot[1].method, "method-2");
    }
}
