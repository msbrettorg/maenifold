#[path = "decay_function/support.rs"]
mod decay_function_support;

use chrono::{Duration, Utc};
use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, TestConfigGuard, acquire_test_lock,
    compute_exponential_decay, compute_exponential_decay_with_half_life,
    compute_exponential_from_date, compute_power_law_from_date,
};

// T-12.6: RTM NFR-1
#[test]
fn exponential_decay_never_exceeds_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [-10, -1, 0, 1, 5, 14, 15, 30, 100, 365, 1000] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/never-exceeds-one",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert!(
            weight <= 1.0,
            "weight should never exceed 1.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn exponential_decay_never_goes_below_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [100, 365, 1000, 3650, 10000] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/never-below-zero",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert!(
            weight >= 0.0,
            "weight should never go below 0.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn exponential_decay_negative_age_treated_as_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/negative-age",
        -5,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_zero_half_life_handled_gracefully() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay_with_half_life(
        &env.cfg,
        "memory://projects/zero-half-life",
        20,
        DEFAULT_GRACE_PERIOD_DAYS,
        0,
    );

    assert!(
        (0.0..=1.0).contains(&weight),
        "zero half-life should still return a bounded weight, got {weight}"
    );
}

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

    assert!(
        weight < 1.0,
        "negative grace should cause immediate decay, got {weight}"
    );
}

#[test]
fn exponential_decay_datetime_api_calculates_correct_age() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let created_date = Utc::now() - Duration::days(10);

    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-exponential",
        created_date,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert_eq!(weight, 1.0);
}

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

#[test]
fn power_law_decay_datetime_api_calculates_correct_age() {
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
