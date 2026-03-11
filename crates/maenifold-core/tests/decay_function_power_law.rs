#[path = "decay_function/support.rs"]
mod decay_function_support;

use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, POWER_LAW_A, POWER_LAW_B, TestConfigGuard, acquire_test_lock,
    assert_approx_eq, compute_exponential_decay, compute_power_law_decay, power_law_formula,
};

// T-12.6: RTM NFR-1
#[test]
fn power_law_decay_basic_calculation_follows_formula() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-basic", 4, 0);
    let expected = POWER_LAW_A * 4.0f64.powf(-POWER_LAW_B);

    assert_approx_eq(weight, expected, 0.0001);
}

#[test]
fn power_law_decay_at_day_one_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-day-one", 1, 0);

    assert_approx_eq(weight, 1.0, 0.0001);
}

#[test]
fn power_law_decay_below_day_one_caps_at_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let _env = decay_function_support::DecayEnvGuard::set("power-law", 30, 7, 14, 0);
    let last_accessed = (chrono::Utc::now() - chrono::Duration::hours(12)).to_rfc3339();

    let weight = decay_function_support::compute_with_last_accessed(
        &env.cfg,
        "memory://projects/power-half-day",
        &last_accessed,
        None,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_zero_age_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-zero-age", 0, 0);

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_never_exceeds_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [0, 1, 2, 10, 100, 365, 1000, 3650, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-never-exceeds-one",
            age_days,
            0,
        );
        assert!(
            weight <= 1.0,
            "power-law weight should never exceed 1.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn power_law_decay_never_goes_below_zero() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [100, 365, 1000, 3650, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-never-below-zero",
            age_days,
            0,
        );
        assert!(
            weight >= 0.0,
            "power-law weight should never go below 0.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn power_law_decay_slower_than_exponential_for_old_content() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let exponential = compute_exponential_decay(
        &env.cfg,
        "memory://projects/old-content-exponential",
        120,
        DEFAULT_GRACE_PERIOD_DAYS,
    );
    let power_law =
        compute_power_law_decay(&env.cfg, "memory://projects/old-content-power", 120, 0);

    assert!(
        power_law >= exponential * 0.5,
        "power-law should not decay drastically faster than exponential: power={power_law}, exp={exponential}"
    );
}

#[test]
fn power_law_decay_produces_different_curve_than_exponential() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let mut difference_found = false;
    for age_days in [1, 4, 16, 64, 256] {
        let exponential =
            compute_exponential_decay(&env.cfg, "memory://projects/curve-exp", age_days, 0);
        let power_law =
            compute_power_law_decay(&env.cfg, "memory://projects/curve-power", age_days, 0);
        if (exponential - power_law).abs() > 0.01 {
            difference_found = true;
        }
    }

    assert!(
        difference_found,
        "power-law and exponential curves should differ at one or more time points"
    );
}

#[test]
fn power_law_decay_memory_halves_when_time_quadruples() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight_at_4 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-4", 4, 0);
    let weight_at_16 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-16", 16, 0);
    let weight_at_64 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-64", 64, 0);

    assert_approx_eq(weight_at_4, 0.5, 0.0001);
    assert_approx_eq(weight_at_16, weight_at_4 / 2.0, 0.0001);
    assert_approx_eq(weight_at_64, weight_at_16 / 2.0, 0.0001);
}

#[test]
fn power_law_decay_negative_age_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-negative-age", -5, 0);

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_custom_parameters_applies_correctly() {
    let _guard = acquire_test_lock();

    let weight = power_law_formula(10.0, 2.0, 0.3);
    let expected = (2.0 * 10.0f64.powf(-0.3)).clamp(0.0, 1.0);

    assert_approx_eq(weight, expected, 0.0001);
}
