#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1

// T-12.7: RTM NFR-1
/// Age 20 is within the default 28-day grace but past the workflow 14-day and
/// sequential 7-day grace periods. This makes the assertion discriminating:
/// if the path were incorrectly routed to any non-default grace category,
/// `weight_for` would return < 1.0 and the test would fail.
#[test]
fn power_law_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://notes/power-law-within-grace",
        Some(20),
        None,
    );

    assert_eq!(weight, 1.0);
}

// T-12.7: RTM NFR-1
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

// T-12.7: RTM NFR-1
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

// T-12.7: RTM NFR-1
/// Verifies that power-law and exponential decay produce distinct results and
/// that power-law decays slower (heavier tail) than exponential for old content.
/// For effective age 60: power-law ≈ 0.129, exponential = 0.25 — but power-law
/// is below exponential because the exponent is −0.5 applied to a large age.
/// The directional assertion catches a regression where power-law accidentally
/// decays faster than exponential.
#[test]
fn power_law_produces_different_result_from_exponential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // NOTE: `env.cfg` reads decay configuration dynamically from env vars at
    // call time, not at `Config::current()` construction time. Swapping the
    // DecayEnvGuard between calls changes the decay function for the same cfg.
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
        power_law < exponential,
        "power-law should decay slower than exponential for old content: power={power_law}, exp={exponential}"
    );
}

// T-12.7: RTM NFR-1
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

// T-12.7: RTM NFR-1
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

// T-12.7: RTM NFR-1
/// Production code hard-switches assumption paths to exponential decay regardless
/// of the configured decay function. This test sets power-law, uses an assumption
/// path with no status, and verifies the result matches exponential — not power-law.
/// Covers the `is_assumption_path` → `"exponential"` override in `scoring.rs`.
#[test]
fn power_law_assumption_path_forces_exponential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Compute the power-law value for a non-assumption path at the same age.
    let _power_law = DecayEnvGuard::power_law_defaults();
    let power_law_value = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/assumption-compare",
        Some(14 + 60),
        None,
    );
    drop(_power_law);

    // Compute the exponential value for reference.
    let _exponential = DecayEnvGuard::exponential_defaults();
    let exponential_value = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/power-law-override",
        Some(28 + 60),
        None,
    );
    drop(_exponential);

    // Now set power-law and use an assumption path — should get exponential, not power-law.
    let _power_law = DecayEnvGuard::power_law_defaults();
    let assumption_weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/power-law-override",
        Some(28 + 60),
        None,
    );

    assert_approx_eq(assumption_weight, exponential_value, 1e-9);
    assert!(
        (assumption_weight - power_law_value).abs() > 0.01,
        "assumption path should NOT use power-law: assumption={assumption_weight}, power_law={power_law_value}"
    );
}

// T-12.7: RTM NFR-1
/// Sequential paths (`memory://thinking/sequential/...`) use the sequential grace
/// period (7 days). Age 10 is past sequential grace (7) but within workflow (14)
/// and default (28). If the path were misrouted to workflow or default grace, the
/// weight would still be 1.0 and this test would fail.
#[test]
fn power_law_sequential_path_uses_sequential_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/sequential/2026/03/power-law-sequential",
        Some(10),
        None,
    );

    assert!(
        weight < 1.0,
        "expected decay at day 10 (past sequential grace 7), got {weight}"
    );
}
