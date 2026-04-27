//! Per-conversation FIFO prompt queue.
//!
//! Ports the queueing model from `agency-cowork/skills/teams/scripts/monitor/
//! prompt_queue.py` (L112-637): instead of dispatching matched Teams messages
//! immediately, each message is enqueued onto a per-conversation channel and
//! a dedicated worker drains that channel sequentially. This guarantees that
//! prompts in the same conversation are processed in arrival order while
//! distinct conversations run in parallel.
//!
//! Mapping decisions vs the Python source:
//!  - `asyncio.Queue` -> `tokio::sync::mpsc::channel` with `try_send` for
//!    non-blocking backpressure (returns [`QueueFull`]).
//!  - PtyBridge / `_ensure_session` is intentionally **not** ported. Session
//!    reuse is the responsibility of the supervisor/dispatcher (T-238.61).
//!  - Idle eviction is exposed as an explicit [`PromptQueue::reap_idle`] call
//!    so the supervisor decides cadence; no background reaper task is spawned.
//!
//! Not yet wired into `supervisor.rs` — that requires the per-conversation
//! session work in T-238.61.
#![allow(dead_code)]


use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

/// Default per-conversation channel capacity (matches Python `pty_queue_max`).
pub const DEFAULT_QUEUE_SIZE: usize = 10;
/// Default idle timeout before a conversation worker is reaped (5 minutes,
/// matches Python `pty_idle_timeout_minutes * 60`).
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

/// One enqueued prompt awaiting dispatch.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub prompt: String,
    pub conversation_id: String,
    pub sender_name: String,
    pub sender_mri: String,
}

/// Returned by [`PromptQueue::enqueue`] when the per-conversation channel is
/// at capacity. The caller (eventually `process_messages` in the supervisor)
/// is expected to log and optionally surface a "queue full" reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueFull {
    pub conversation_id: String,
    pub capacity: usize,
}

impl std::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "prompt queue full for conversation {} (capacity {})",
            self.conversation_id, self.capacity
        )
    }
}

impl std::error::Error for QueueFull {}

/// Async dispatch function invoked by each per-conversation worker. Returning
/// an error does not kill the worker; the next item is still processed.
pub type DispatchFn = Arc<
    dyn Fn(QueueItem) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

struct ConversationQueue {
    tx: mpsc::Sender<QueueItem>,
    last_activity: Instant,
}

/// Per-conversation FIFO queue + worker registry.
///
/// Not internally synchronized — callers (the supervisor loop) own this and
/// drive both `enqueue` and `reap_idle` from a single task.
pub struct PromptQueue {
    conversations: HashMap<String, ConversationQueue>,
    max_queue_size: usize,
    idle_timeout: Duration,
    dispatcher: DispatchFn,
}

impl PromptQueue {
    /// Construct a new queue. `dispatcher` is invoked once per dequeued item
    /// inside the per-conversation worker task.
    pub fn new(max_queue_size: usize, idle_timeout: Duration, dispatcher: DispatchFn) -> Self {
        Self {
            conversations: HashMap::new(),
            max_queue_size: max_queue_size.max(1),
            idle_timeout,
            dispatcher,
        }
    }

    /// Enqueue a prompt for its conversation, spawning a worker on first use.
    /// Returns [`QueueFull`] if the per-conversation channel is at capacity.
    pub fn enqueue(&mut self, item: QueueItem) -> Result<(), QueueFull> {
        let key = item.conversation_id.clone();
        let cap = self.max_queue_size;

        // Drop any conversation whose worker has exited (channel closed) so
        // we re-create it cleanly on the next message.
        if let Some(existing) = self.conversations.get(&key) {
            if existing.tx.is_closed() {
                self.conversations.remove(&key);
            }
        }

        let entry = self.conversations.entry(key.clone()).or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<QueueItem>(cap);
            spawn_worker(rx, self.dispatcher.clone());
            ConversationQueue {
                tx,
                last_activity: Instant::now(),
            }
        });

        match entry.tx.try_send(item) {
            Ok(()) => {
                entry.last_activity = Instant::now();
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => Err(QueueFull {
                conversation_id: key,
                capacity: cap,
            }),
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Worker died between the freshness check and try_send; drop
                // the registry entry so the next call re-spawns.
                self.conversations.remove(&key);
                Err(QueueFull {
                    conversation_id: key,
                    capacity: cap,
                })
            }
        }
    }

    /// Number of conversations with a live worker.
    pub fn active_conversations(&self) -> usize {
        self.conversations.len()
    }

    /// Drop conversations whose last activity is older than `idle_timeout`.
    /// Dropping the [`mpsc::Sender`] closes the channel and the worker exits
    /// once it has drained any in-flight items.
    pub fn reap_idle(&mut self) {
        let cutoff = self.idle_timeout;
        self.conversations
            .retain(|_, q| q.last_activity.elapsed() < cutoff);
    }
}

fn spawn_worker(mut rx: mpsc::Receiver<QueueItem>, dispatcher: DispatchFn) {
    tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            // Errors are swallowed — the worker must keep draining so that a
            // single failed dispatch does not strand subsequent prompts.
            let _ = dispatcher(item).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn item(conv: &str, prompt: &str) -> QueueItem {
        QueueItem {
            prompt: prompt.into(),
            conversation_id: conv.into(),
            sender_name: "alice".into(),
            sender_mri: "8:orgid:alice".into(),
        }
    }

    fn recording_dispatcher() -> (DispatchFn, Arc<Mutex<Vec<(String, String)>>>) {
        let log: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        let f: DispatchFn = Arc::new(move |i: QueueItem| {
            let log = log2.clone();
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                log.lock().unwrap().push((i.conversation_id, i.prompt));
                Ok(())
            })
        });
        (f, log)
    }

    #[tokio::test]
    async fn prompts_for_same_conversation_run_sequentially() {
        let (disp, log) = recording_dispatcher();
        let mut q = PromptQueue::new(8, Duration::from_secs(60), disp);
        q.enqueue(item("c1", "a")).unwrap();
        q.enqueue(item("c1", "b")).unwrap();
        q.enqueue(item("c1", "c")).unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        let observed: Vec<String> = log.lock().unwrap().iter().map(|(_, p)| p.clone()).collect();
        assert_eq!(observed, vec!["a", "b", "c"]);
        assert_eq!(q.active_conversations(), 1);
    }

    #[tokio::test]
    async fn queue_full_returns_error() {
        // Dispatcher blocks long enough that the channel saturates.
        let f: DispatchFn = Arc::new(|_| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok(())
            })
        });
        let mut q = PromptQueue::new(1, Duration::from_secs(60), f);
        // First send is taken immediately by the worker (buffer empties),
        // second fills the buffer, third should reject.
        q.enqueue(item("c1", "a")).unwrap();
        // Give the worker a tick to pick up "a".
        tokio::time::sleep(Duration::from_millis(10)).await;
        q.enqueue(item("c1", "b")).unwrap();
        let err = q.enqueue(item("c1", "c")).unwrap_err();
        assert_eq!(err.capacity, 1);
        assert_eq!(err.conversation_id, "c1");
    }

    #[tokio::test]
    async fn reap_idle_drops_stale_conversations() {
        let (disp, _log) = recording_dispatcher();
        let mut q = PromptQueue::new(8, Duration::from_millis(50), disp);
        q.enqueue(item("c1", "a")).unwrap();
        assert_eq!(q.active_conversations(), 1);
        tokio::time::sleep(Duration::from_millis(80)).await;
        q.reap_idle();
        assert_eq!(q.active_conversations(), 0);
    }

    #[tokio::test]
    async fn dispatcher_error_does_not_kill_worker() {
        let count = Arc::new(Mutex::new(0u32));
        let count2 = count.clone();
        let f: DispatchFn = Arc::new(move |_| {
            let count = count2.clone();
            Box::pin(async move {
                let mut g = count.lock().unwrap();
                *g += 1;
                if *g == 1 { Err("boom".into()) } else { Ok(()) }
            })
        });
        let mut q = PromptQueue::new(4, Duration::from_secs(60), f);
        q.enqueue(item("c1", "a")).unwrap();
        q.enqueue(item("c1", "b")).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*count.lock().unwrap(), 2);
    }
}
