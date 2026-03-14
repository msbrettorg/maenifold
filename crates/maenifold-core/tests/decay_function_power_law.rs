#[path = "decay_function/support.rs"]
mod decay_function_support;

use decay_function_support::{
    POWER_LAW_A, POWER_LAW_B, TestConfigGuard, acquire_test_lock, assert_approx_eq,
    compute_exponential_decay, compute_power_law_decay,
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

// T-12.6: RTM NFR-1 — MERGE: absorbed below_day_one_caps_at_one (same grace-period branch)
#[test]
fn power_law_decay_zero_age_returns_weight_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    // age_days == 0 hits the grace-period gate directly
    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-zero-age", 0, 0);
    assert_eq!(weight, 1.0);

    // Sub-day timestamp (12 hours old) also yields age_days=0, same branch
    let _env = decay_function_support::DecayEnvGuard::set("power-law", 30, 7, 14, 0);
    let last_accessed = (chrono::Utc::now() - chrono::Duration::hours(12)).to_rfc3339();
    let weight_sub_day = decay_function_support::compute_with_last_accessed(
        &env.cfg,
        "memory://projects/power-half-day",
        &last_accessed,
        None,
    );
    assert_eq!(weight_sub_day, 1.0);
}

// T-12.6: RTM NFR-1 — MERGE: never_exceeds_one + never_goes_below_zero
#[test]
fn power_law_decay_weight_is_bounded_between_zero_and_one() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    for age_days in [0, 1, 2, 10, 100, 365, 1000, 3650, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-bounded",
            age_days,
            0,
        );
        assert!(
            (0.0..=1.0).contains(&weight),
            "power-law weight must be in [0.0, 1.0] for age={age_days}, got {weight}"
        );
    }
}

// T-12.6: RTM NFR-1 — REWRITE: assert power-law >= exponential for old content
#[test]
fn power_law_decay_slower_than_exponential_for_old_content() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    // Power-law (B=0.5) decays slower than exponential (half-life=30) beyond the
    // crossover point (~100 days with these parameters). This "fat tail" property
    // is the entire motivation for offering power-law decay: old memories retain
    // more weight than under exponential decay.
    // Both use grace_days=0 so the comparison is apples-to-apples.
    for age_days in [120, 180, 365, 1000, 3650] {
        let exponential = compute_exponential_decay(
            &env.cfg,
            "memory://projects/old-content-exponential",
            age_days,
            0,
        );
        let power_law =
            compute_power_law_decay(&env.cfg, "memory://projects/old-content-power", age_days, 0);

        assert!(
            power_law >= exponential,
            "power-law should retain more weight than exponential at age={age_days}: \
             power_law={power_law}, exponential={exponential}"
        );
    }
}

// T-12.6: RTM NFR-1 — REWRITE: verify curve shape via expected values, not mere distinctness
#[test]
fn power_law_decay_curve_matches_expected_values() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    // Production formula: weight = A * effective_age^(-B) where A=1.0, B=0.5
    // effective_age = max(age_days, 1), so for age >= 1: weight = age^(-0.5)
    let expected: [(i64, f64); 5] = [
        (1, 1.0),               // 1^(-0.5) = 1.0
        (4, 0.5),               // 4^(-0.5) = 0.5
        (16, 0.25),             // 16^(-0.5) = 0.25
        (64, 0.125),            // 64^(-0.5) = 0.125
        (256, 0.0625),          // 256^(-0.5) = 0.0625
    ];

    for (age_days, expected_weight) in expected {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-curve-shape",
            age_days,
            0,
        );
        assert_approx_eq(weight, expected_weight, 0.0001);
    }
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
