#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://notes/power-law-within-grace",
        Some(5),
        None,
    );

    assert_eq!(weight, 1.0);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_at_grace_boundary_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-boundary",
        Some(14),
        None,
    );

    assert_eq!(weight, 1.0);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_past_grace_period_calculates_effective_age_decay() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-decay",
        Some(14 + 4),
        None,
    );

    assert_approx_eq(weight, 0.5, 1e-9);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_produces_different_result_from_exponential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let _power_law = DecayEnvGuard::power_law_defaults();
    let power_law = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-vs-exponential",
        Some(14 + 60),
        None,
    );
    drop(_power_law);

    let _exponential = DecayEnvGuard::exponential_defaults();
    let exponential = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-vs-exponential",
        Some(14 + 60),
        None,
    );

    assert!(
        (power_law - exponential).abs() > 0.01,
        "expected power_law={power_law} and exponential={exponential} to diverge"
    );
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_function_string_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let _lower = DecayEnvGuard::with_values("power-law", 30, 7, 14, 28);
    let lower = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );
    drop(_lower);

    let _upper = DecayEnvGuard::with_values("POWER-LAW", 30, 7, 14, 28);
    let upper = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );
    drop(_upper);

    let _mixed = DecayEnvGuard::with_values("Power-Law", 30, 7, 14, 28);
    let mixed = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );

    assert_approx_eq(upper, lower, 1e-9);
    assert_approx_eq(mixed, lower, 1e-9);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_weight_is_bounded_between_zero_and_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    for age_days in [0, 1, 14, 15, 30, 100, 365, 3650] {
        let weight = weight_for(
            &env.cfg,
            "memory://thinking/workflow/2026/03/bounded",
            Some(age_days),
            None,
        );
        assert!(
            (0.0..=1.0).contains(&weight),
            "expected bounded weight for age_days={age_days}, got {weight}"
        );
    }
}
