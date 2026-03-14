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

// T-7.8: RTM FR-7.8 — Validated status is permanently pinned to weight 1.0
// regardless of age. A representative spread of ages from recent (1 day) to
// very old (730 days) proves the invariant holds across the full range.
#[test]
fn validated_status_never_decays() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    for days in [1, 90, 365, 730] {
        let weight = weight_for(
            &env.cfg,
            &format!("memory://assumptions/2026/03/validated-age-{days}"),
            Some(days),
            Some("validated"),
        );

        assert_eq!(
            weight, 1.0,
            "validated status must always return 1.0 regardless of age; failed at {days} days"
        );
    }
}

// T-7.8: RTM FR-7.8 — Case variants of "validated" must behave identically to
// the lowercase canonical form. Uses the comparison-of-two-outputs oracle:
// each variant is compared against the lowercase baseline with a tight tolerance,
// isolating case-sensitivity from the "validated-always-returns-one" invariant.
#[test]
fn validated_status_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let baseline = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-baseline",
        Some(90),
        Some("validated"),
    );

    for variant in ["VALIDATED", "Validated", "vAlIdAtEd"] {
        let weight = weight_for(
            &env.cfg,
            &format!("memory://assumptions/2026/03/validated-case-{variant}"),
            Some(90),
            Some(variant),
        );

        assert_approx_eq(weight, baseline, 1e-9);
    }
}

// T-7.8: RTM FR-7.8 — Invalidated files within the grace period (7 days for
// exponential_defaults) have not yet begun decaying, so weight must be 1.0.
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

// T-7.8: RTM FR-7.8 — After the grace period, invalidated files decay with an
// accelerated half-life. With exponential_defaults():
//   - grace period for invalidated (sequential) = 7 days
//   - global half_life = 30 days
//   - invalidated acceleration factor = ~2.14× (30 / 14 ≈ effective half-life of 14 days)
// At age 21 days: 7 days grace + 14 days of decay = exactly one effective
// half-life past grace, so weight ≈ 0.5.
#[test]
fn invalidated_status_past_grace_period_decays_aggressively() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-half-life",
        Some(7 + 14), // 7 days grace + 14 days = one effective half-life for invalidated
        Some("invalidated"),
    );

    assert_approx_eq(weight, 0.5, 0.01);
}

// T-7.8: RTM FR-7.8 — Case variants of "invalidated" must produce identical
// decay weights. Uses the comparison-of-two-outputs oracle: lower vs upper are
// compared with a tight tolerance, isolating case-sensitivity from decay math.
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

// T-7.8: RTM FR-7.8 — Invalidated files must decay faster than files with
// no special status. The "active" status string is treated as an unrecognized
// status by compute_decay_weight, falling through to the default decay curve
// (no acceleration). This test confirms the semantic contract: invalidation
// accelerates decay relative to the baseline/default behavior.
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
    // "active" is not a recognized status — it falls through to the default
    // decay curve with no acceleration, serving as the baseline.
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
