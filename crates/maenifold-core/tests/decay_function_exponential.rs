#[path = "decay_function/support.rs"]
mod decay_function_support;

use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, TestConfigGuard, acquire_test_lock,
    assert_approx_eq, compute_exponential_decay,
};

// T-12.6: RTM NFR-1 — Boundary test for grace-period fence-post (< vs <=).
// Interior-point tests (age=0, age=5) removed: subsumed by this boundary guard.
#[test]
fn exponential_decay_exactly_at_grace_period_boundary_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/grace-boundary",
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

// T-12.6: RTM NFR-1 — Parameterized half-life periodicity: 2^(-N) at N half-lives past grace.
#[test]
fn exponential_decay_n_half_lives_past_grace_returns_correct_weight() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for (n, expected) in [(1, 0.5), (2, 0.25), (3, 0.125)] {
        let age_days = DEFAULT_GRACE_PERIOD_DAYS + (n * DEFAULT_HALF_LIFE_DAYS);
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/n-half-lives",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );
        assert_approx_eq(weight, expected, 0.0001);
    }
}

// T-12.6: RTM NFR-1
#[test]
fn exponential_decay_half_of_half_life_past_grace_returns_intermediate_weight() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (DEFAULT_HALF_LIFE_DAYS / 2);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/half-half-life",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    let expected = 0.5f64.powf(0.5);
    assert_approx_eq(weight, expected, 0.0001);
}

#[test]
fn exponential_decay_very_old_content_approaches_zero_but_never_reaches() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/very-old",
        3650,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert!(
        weight > 0.0,
        "very old content should remain asymptotically above zero"
    );
    assert!(
        weight < 0.001,
        "very old content should decay to a very small value"
    );
}
