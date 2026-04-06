#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(super) struct CircuitBreaker {
    threshold: u32,
    cooldown: Duration,
    targets: Mutex<HashMap<String, TargetState>>,
}

struct TargetState {
    consecutive_transport_failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    pub(super) fn new(threshold: u32, cooldown: Duration) -> Self {
        Self {
            threshold,
            cooldown,
            targets: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn record_failure(&self, target: &str, is_transport: bool) {
        if !is_transport {
            return;
        }

        let mut targets = self.targets.lock().unwrap();
        let state = targets.entry(target.to_string()).or_insert(TargetState {
            consecutive_transport_failures: 0,
            opened_at: None,
        });

        if state
            .opened_at
            .is_some_and(|opened_at| opened_at.elapsed() >= self.cooldown)
        {
            state.consecutive_transport_failures = 0;
            state.opened_at = None;
        }

        state.consecutive_transport_failures += 1;
        if state.consecutive_transport_failures >= self.threshold {
            state.opened_at = Some(Instant::now());
        }
    }

    pub(super) fn record_success(&self, target: &str) {
        let mut targets = self.targets.lock().unwrap();
        if let Some(state) = targets.get_mut(target) {
            state.consecutive_transport_failures = 0;
            state.opened_at = None;
        }
    }

    pub(super) fn is_open(&self, target: &str) -> bool {
        let mut targets = self.targets.lock().unwrap();
        match targets.get_mut(target) {
            Some(state) => match state.opened_at {
                Some(opened_at) if opened_at.elapsed() < self.cooldown => true,
                Some(_) => {
                    state.consecutive_transport_failures = 0;
                    state.opened_at = None;
                    false
                }
                None => false,
            },
            None => false,
        }
    }
}
