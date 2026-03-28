#[allow(clippy::duplicate_mod)]
#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1

// T-7.8: RTM FR-7.8
#[test]
fn active_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-grace",
        Some(10),
        Some("active"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn active_status_past_grace_period_decays_with_thirty_day_half_life() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-half-life",
        Some(14 + 30),
        Some("active"),
    );

    assert_approx_eq(weight, 0.5, 0.01);
}

// T-7.8: RTM FR-7.8
#[test]
fn status_matching_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let cases: &[(&str, &str)] = &[
        ("active", "ACTIVE"),
        ("active", "Active"),
        ("refined", "REFINED"),
        ("refined", "Refined"),
    ];

    for (lower, upper) in cases {
        let lower_weight = weight_for(
            &env.cfg,
            "memory://assumptions/2026/03/case-lower",
            Some(50),
            Some(lower),
        );
        let upper_weight = weight_for(
            &env.cfg,
            "memory://assumptions/2026/03/case-upper",
            Some(50),
            Some(upper),
        );

        assert_approx_eq(upper_weight, lower_weight, 1e-9);
    }
}

// T-7.8: RTM FR-7.8
#[test]
fn refined_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-grace",
        Some(5),
        Some("refined"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn refined_status_uses_same_parameters_as_active() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let refined = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-vs-active",
        Some(60),
        Some("refined"),
    );
    let active = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-vs-refined",
        Some(60),
        Some("active"),
    );

    assert_approx_eq(refined, active, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn unknown_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let unknown = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/unknown-status",
        Some(60),
        Some("unknown-garbage"),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/absent-status",
        Some(60),
        None,
    );

    assert_approx_eq(unknown, absent, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn empty_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let empty = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/empty-status",
        Some(60),
        Some(""),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/empty-status-baseline",
        Some(60),
        None,
    );

    assert_approx_eq(empty, absent, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn absent_status_uses_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/no-status",
        Some(60),
        None,
    );

    // T-12.7: RTM NFR-1 — pin to the formula with configured defaults:
    // grace = 28 (MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT), half_life = 30, age = 60
    // weight = 0.5^((60 - 28) / 30) ≈ 0.479
    let expected = 0.5f64.powf((60.0 - 28.0) / 30.0);
    assert_approx_eq(absent, expected, 0.01);
}

// T-7.8: RTM FR-7.8
#[test]
fn whitespace_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let whitespace = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/whitespace-status",
        Some(60),
        Some("   "),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/whitespace-status-baseline",
        Some(60),
        None,
    );

    assert_approx_eq(whitespace, absent, 1e-9);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn missing_last_accessed_defaults_to_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "memory://notes/no-accessed", None, None);

    assert_eq!(weight, 1.0);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn recent_last_accessed_keeps_weight_at_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "memory://notes/recent-access", Some(1), None);

    assert_eq!(weight, 1.0);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn recent_last_accessed_produces_higher_weight_than_old_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let old_weight = weight_for(&env.cfg, "memory://notes/older-access", Some(60), None);
    let recent_weight = weight_for(&env.cfg, "memory://notes/recent-access", Some(1), None);

    assert!(
        recent_weight > old_weight,
        "expected recent_weight={recent_weight} to exceed old_weight={old_weight}"
    );
}
