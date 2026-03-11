#[path = "decay_function/support.rs"]
mod decay_function_support;

use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, TestConfigGuard, acquire_test_lock,
    assert_approx_eq, compute_exponential_decay,
};

// T-12.6: RTM NFR-1
#[test]
fn exponential_decay_within_grace_period_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/within-grace",
        5,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_zero_age_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/zero-age",
        0,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

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

#[test]
fn exponential_decay_one_half_life_past_grace_returns_weight_half() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + DEFAULT_HALF_LIFE_DAYS;
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/one-half-life",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.5, 0.0001);
}

#[test]
fn exponential_decay_two_half_lives_past_grace_returns_weight_quarter() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (2 * DEFAULT_HALF_LIFE_DAYS);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/two-half-lives",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.25, 0.0001);
}

#[test]
fn exponential_decay_three_half_lives_past_grace_returns_weight_eighth() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (3 * DEFAULT_HALF_LIFE_DAYS);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/three-half-lives",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.125, 0.0001);
}

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
