use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;
use std::collections::HashSet;

pub const DEFAULT_THROTTLE_WINDOW: Duration = Duration::from_millis(250);

#[derive(Clone, Debug)]
pub struct TrailingThrottle {
    window: Duration,
    state: Arc<Mutex<ThrottleState>>,
}

#[derive(Debug)]
struct ThrottleState {
    last_dispatch_at: Option<Instant>,
    current_epoch: u64,
    pending_trailing_epochs: HashSet<u64>,
    pending_dispatches: usize,
}

impl Default for TrailingThrottle {
    fn default() -> Self {
        Self::new(DEFAULT_THROTTLE_WINDOW)
    }
}

impl TrailingThrottle {
    pub fn new(window: Duration) -> Self {
        let normalized_window = if window.is_zero() {
            Duration::from_millis(1)
        } else {
            window
        };

        Self {
            window: normalized_window,
            state: Arc::new(Mutex::new(ThrottleState {
                last_dispatch_at: None,
                current_epoch: 0,
                pending_trailing_epochs: HashSet::new(),
                pending_dispatches: 0,
            })),
        }
    }

    pub fn window(&self) -> Duration {
        self.window
    }

    /// Registers a signal with trailing-edge coalescing semantics.
    ///
    /// Returns `true` when this signal should dispatch immediately.
    /// Returns `false` when the signal was coalesced into a pending trailing dispatch.
    pub async fn signal(&self) -> bool {
        let now = Instant::now();
        let mut state = self.state.lock().await;

        let Some(last_dispatch_at) = state.last_dispatch_at else {
            state.last_dispatch_at = Some(now);
            state.current_epoch = state.current_epoch.saturating_add(1);
            return true;
        };

        let elapsed = now.saturating_duration_since(last_dispatch_at);
        if elapsed >= self.window {
            state.last_dispatch_at = Some(now);
            state.current_epoch = state.current_epoch.saturating_add(1);
            return true;
        }

        let epoch = state.current_epoch;
        if state.pending_trailing_epochs.contains(&epoch) {
            return false;
        }

        let remaining = self.window - elapsed;
        state.pending_trailing_epochs.insert(epoch);

        let shared_state = Arc::clone(&self.state);
        tokio::spawn(async move {
            tokio::time::sleep(remaining).await;
            let mut state = shared_state.lock().await;
            if state.pending_trailing_epochs.remove(&epoch) {
                state.pending_dispatches = state.pending_dispatches.saturating_add(1);
                state.last_dispatch_at = Some(Instant::now());
                state.current_epoch = state.current_epoch.saturating_add(1);
            }
        });

        false
    }

    /// Non-blocking check for a coalesced trailing dispatch becoming ready.
    ///
    /// Returns `true` exactly once per trailing dispatch window.
    pub async fn take_trailing_dispatch(&self) -> bool {
        let mut state = self.state.lock().await;
        if state.pending_dispatches == 0 {
            return false;
        }

        state.pending_dispatches -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{TrailingThrottle, DEFAULT_THROTTLE_WINDOW};
    use std::time::Duration;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn first_signal_dispatches_immediately() {
        let throttle = TrailingThrottle::default();
        assert_eq!(throttle.window(), DEFAULT_THROTTLE_WINDOW);
        assert!(throttle.signal().await);
        assert!(!throttle.take_trailing_dispatch().await);
    }

    #[tokio::test]
    async fn rapid_signals_coalesce_to_one_trailing_dispatch() {
        let throttle = TrailingThrottle::new(Duration::from_millis(25));

        assert!(throttle.signal().await);
        for _ in 0..8 {
            assert!(!throttle.signal().await);
        }

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(throttle.take_trailing_dispatch().await);
        assert!(!throttle.take_trailing_dispatch().await);
    }

    #[tokio::test]
    async fn trailing_edge_sequence_matches_expected_windows() {
        let throttle = TrailingThrottle::new(Duration::from_millis(40));

        assert!(throttle.signal().await); // t=0 immediate
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!throttle.signal().await); // coalesced into trailing at ~40ms

        tokio::time::sleep(Duration::from_millis(35)).await;
        assert!(throttle.take_trailing_dispatch().await); // trailing from t=10ms signal

        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(!throttle.signal().await); // within trailing window; schedule new trailing
        tokio::time::sleep(Duration::from_millis(45)).await;
        assert!(throttle.take_trailing_dispatch().await);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn boundary_signal_does_not_cancel_prior_trailing_dispatch() {
        let throttle = TrailingThrottle::new(Duration::from_millis(250));

        assert!(throttle.signal().await); // t=0 immediate
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!throttle.signal().await); // schedules trailing for first window

        // Block the current-thread runtime past the window boundary so the sleeper
        // cannot run before this boundary signal executes.
        std::thread::sleep(Duration::from_millis(300)); // t=300+
        assert!(throttle.signal().await); // new immediate after window boundary

        tokio::task::yield_now().await;
        assert!(throttle.take_trailing_dispatch().await); // prior window trailing still fires
        assert!(!throttle.take_trailing_dispatch().await);
    }

    #[tokio::test]
    async fn concurrent_callers_still_coalesce_to_one_trailing_dispatch() {
        let throttle = TrailingThrottle::new(Duration::from_millis(25));
        let shared = throttle.clone();

        let mut join_set = JoinSet::new();
        for _ in 0..32 {
            let throttle = shared.clone();
            join_set.spawn(async move { throttle.signal().await });
        }

        let mut immediate_count = 0usize;
        while let Some(result) = join_set.join_next().await {
            if result.expect("signal task panicked") {
                immediate_count = immediate_count.saturating_add(1);
            }
        }

        assert_eq!(immediate_count, 1);
        tokio::time::sleep(Duration::from_millis(35)).await;
        assert!(shared.take_trailing_dispatch().await);
        assert!(!shared.take_trailing_dispatch().await);
    }

    #[tokio::test]
    async fn zero_window_is_clamped_to_one_millisecond() {
        let throttle = TrailingThrottle::new(Duration::ZERO);

        // Zero windows are normalized to a 1ms minimum to avoid busy loops.
        assert_eq!(throttle.window(), Duration::from_millis(1));
        assert!(throttle.signal().await);
        assert!(!throttle.signal().await);
        tokio::time::sleep(Duration::from_millis(2)).await;
        assert!(throttle.take_trailing_dispatch().await);
    }

    #[tokio::test]
    async fn long_pause_resumes_with_immediate_dispatch() {
        let throttle = TrailingThrottle::new(Duration::from_millis(20));

        assert!(throttle.signal().await);
        tokio::time::sleep(Duration::from_millis(40)).await;

        assert!(throttle.signal().await);
        assert!(!throttle.take_trailing_dispatch().await);
    }
}
