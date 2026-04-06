use std::thread::sleep;
use std::time::Duration;

use super::super::circuit_breaker::CircuitBreaker;

#[test]
fn breaker_starts_closed() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    assert!(!breaker.is_open("agent-a"));
    assert!(!breaker.is_open("agent-b"));
}

#[test]
fn transport_failures_trip_breaker() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);

    assert!(breaker.is_open("agent-a"));
}

#[test]
fn semantic_failures_do_not_trip_breaker() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    for _ in 0..10 {
        breaker.record_failure("agent-a", false);
    }

    assert!(!breaker.is_open("agent-a"));
}

#[test]
fn success_resets_failure_count() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    breaker.record_success("agent-a");
    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);

    assert!(!breaker.is_open("agent-a"));
}

#[test]
fn breaker_scoped_to_target() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);

    assert!(breaker.is_open("agent-a"));
    assert!(!breaker.is_open("agent-b"));
}

#[test]
fn breaker_closes_after_cooldown() {
    let breaker = CircuitBreaker::new(3, Duration::from_millis(20));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    assert!(breaker.is_open("agent-a"));

    sleep(Duration::from_millis(5));
    assert!(
        breaker.is_open("agent-a"),
        "breaker should remain open until cooldown has elapsed"
    );

    sleep(Duration::from_millis(25));
    assert!(!breaker.is_open("agent-a"));
}

#[test]
fn custom_threshold() {
    let breaker = CircuitBreaker::new(5, Duration::from_secs(60));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    assert!(!breaker.is_open("agent-a"));

    breaker.record_failure("agent-a", true);
    breaker.record_failure("agent-a", true);
    assert!(breaker.is_open("agent-a"));
}
