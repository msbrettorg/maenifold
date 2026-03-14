#[path = "decay_function/support.rs"]
mod decay_function_support;

use chrono::{Duration, Utc};
use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, TestConfigGuard, acquire_test_lock,
    compute_exponential_decay, compute_exponential_decay_with_half_life,
    compute_exponential_from_date, compute_power_law_decay, compute_power_law_from_date,
};

// T-12.6: RTM NFR-1
// MERGED from: exponential_decay_never_exceeds_one + exponential_decay_never_goes_below_zero.
// Both originally asserted one half of the unit-interval bound; combined here to assert the
// full [0.0, 1.0] range over the union of their age samples.
#[test]
fn exponential_decay_stays_in_unit_interval() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [-10, -1, 0, 1, 5, 14, 15, 30, 100, 365, 1000, 3650, 10000] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/unit-interval",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert!(
            (0.0..=1.0).contains(&weight),
            "weight must be in [0.0, 1.0] for age={age_days}, got {weight}"
        );
    }
}

// T-12.6: RTM NFR-1
// KEEP: Tests negative age → 1.0 contract. Exact equality (assert_eq) is safe because
// the implementation returns 1.0 via an early-return guard (`if age_days < 0 { return 1.0 }`)
// before entering any floating-point arithmetic. Expanded to multiple negative values to
// rule out a special-case for only one input.
#[test]
fn exponential_decay_negative_age_treated_as_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [-1, -5, -100] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/negative-age",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert_eq!(
            weight, 1.0,
            "negative age {age_days} should return exactly 1.0, got {weight}"
        );
    }
}

// T-12.6: RTM NFR-1
// REWRITE: The implementation returns 0.0 when half_life <= 0 (instant-decay interpretation).
// Previously asserted only (0.0..=1.0).contains(&weight) which would pass for any value in range.
#[test]
fn exponential_decay_zero_half_life_returns_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay_with_half_life(
        &env.cfg,
        "memory://projects/zero-half-life",
        20,
        DEFAULT_GRACE_PERIOD_DAYS,
        0,
    );

    assert_eq!(
        weight, 0.0,
        "zero half-life with age beyond grace should return 0.0 (instant decay), got {weight}"
    );
}

// T-12.6: RTM NFR-1
// REWRITE: Assert the analytically derived value instead of the content-free `weight < 1.0`.
// age=5, grace=-10 → exponent = (5 - (-10)) / 30 = 0.5 → weight = 0.5^0.5 ≈ 0.7071
#[test]
fn exponential_decay_negative_grace_period_treated_as_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay_with_half_life(
        &env.cfg,
        "memory://projects/negative-grace",
        5,
        -10,
        DEFAULT_HALF_LIFE_DAYS,
    );

    let expected = 0.5f64.powf(15.0 / 30.0); // 0.5^0.5 ≈ 0.7071
    decay_function_support::assert_approx_eq(weight, expected, 0.01);
}

// T-12.6: RTM NFR-1
// REWRITE: Renamed from exponential_decay_datetime_api_calculates_correct_age.
// The original only tested a date within the grace window (trivially 1.0). Added a second
// case outside the grace window to verify actual age propagation through the datetime API.
#[test]
fn exponential_decay_datetime_within_grace_returns_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    // Case 1: 10-day-old date is within 14-day grace → 1.0
    let within_grace = Utc::now() - Duration::days(10);
    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-within-grace",
        within_grace,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );
    assert_eq!(weight, 1.0, "within grace period should return 1.0");

    // Case 2: 44-day-old date is beyond 14-day grace.
    // effective_age = 44 - 14 = 30, exponent = 30/30 = 1.0, weight = 0.5^1.0 = 0.5
    let beyond_grace = Utc::now() - Duration::days(44);
    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-beyond-grace",
        beyond_grace,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );
    decay_function_support::assert_approx_eq(weight, 0.5, 0.01);
}

// T-12.6: RTM NFR-1
// KEEP
#[test]
fn exponential_decay_datetime_api_future_date_returns_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let future_date = Utc::now() + Duration::days(5);

    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-future",
        future_date,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert_eq!(weight, 1.0);
}

// T-12.6: RTM NFR-1
// KEEP (rename): Renamed from power_law_decay_datetime_api_calculates_correct_age.
// age=4 days, grace=0, exponent=-0.5 → effective_age=4 → 4^(-0.5) = 0.5
#[test]
fn power_law_decay_datetime_age_four_no_grace_yields_half() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let created_date = Utc::now() - Duration::days(4);

    let weight = compute_power_law_from_date(
        &env.cfg,
        "memory://projects/datetime-power",
        created_date,
        0,
    );

    decay_function_support::assert_approx_eq(weight, 0.5, 0.01);
}

// --- Power-law boundary coverage (P1) ---
// The exponential branch had five dedicated boundary tests; the power-law branch had only one.
// These tests close the asymmetry.

// T-12.6: RTM NFR-1
#[test]
fn power_law_decay_stays_in_unit_interval() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [-10, -1, 0, 1, 5, 14, 15, 30, 100, 365, 1000, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-law-unit-interval",
            age_days,
            0,
        );

        assert!(
            (0.0..=1.0).contains(&weight),
            "power-law weight must be in [0.0, 1.0] for age={age_days}, got {weight}"
        );
    }
}

// T-12.6: RTM NFR-1
#[test]
fn power_law_decay_negative_age_returns_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [-1, -5, -100] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-law-neg-age",
            age_days,
            0,
        );

        assert_eq!(
            weight, 1.0,
            "power-law: negative age {age_days} should return 1.0, got {weight}"
        );
    }
}

// T-12.6: RTM NFR-1
// Verifies non-trivial power-law decay: age=100, grace=0 → effective_age=100 → 100^(-0.5) = 0.1
#[test]
fn power_law_decay_outside_grace_decays_correctly() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(
        &env.cfg,
        "memory://projects/power-law-outside-grace",
        100,
        0,
    );

    let expected = 100.0f64.powf(-0.5); // 0.1
    decay_function_support::assert_approx_eq(weight, expected, 0.01);
}
