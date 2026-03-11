#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1
// These tests port the C# DecayCalculator coverage scenarios onto Rust's single
// `compute_decay_weight(file_path, last_accessed, config, status)` API surface.
// Rust does not expose the C# overload pair or a separate `created` timestamp,
// so overload-only cases are represented through equivalent single-entrypoint
// scenarios and `last_accessed`-driven reference dates.

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_returns_one_when_new() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-new",
        Some(1),
        Some("validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_returns_one_when_very_old() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-old",
        Some(365),
        Some("validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_is_case_insensitive_uppercase() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-upper",
        Some(90),
        Some("VALIDATED"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_is_case_insensitive_mixed_case() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-mixed",
        Some(60),
        Some("Validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-grace",
        Some(3),
        Some("invalidated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_past_grace_period_decays_aggressively() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-half-life",
        Some(7 + 14),
        Some("invalidated"),
    );

    assert_approx_eq(weight, 0.5, 0.01);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let lower = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-lower",
        Some(30),
        Some("invalidated"),
    );
    let upper = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-upper",
        Some(30),
        Some("INVALIDATED"),
    );

    assert_approx_eq(upper, lower, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_decays_faster_than_active() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let invalidated = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-vs-active",
        Some(30),
        Some("invalidated"),
    );
    let active = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-vs-invalidated",
        Some(30),
        Some("active"),
    );

    assert!(
        invalidated < active,
        "expected invalidated={invalidated} to decay faster than active={active}"
    );
}
