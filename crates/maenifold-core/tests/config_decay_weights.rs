#[path = "config_decay/support.rs"]
mod support;

// T-12.5: RTM NFR-1
use maenifold_core::memory;
use support::{
    acquire_test_lock, apply_decay_env, assert_approx_eq, clear_decay_env, config_summary,
    isolated_env, rfc3339_days_ago,
};

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
// MERGE of T1–T3: parameterized over path types; adds grace+1 off-by-one coverage
#[test]
fn compute_decay_weight_path_type_uses_its_config_grace_period() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    let cases: [(&str, i64); 3] = [
        (
            "memory://thinking/sequential/2026/03/s",
            env.cfg.decay_grace_days_sequential(),
        ),
        (
            "memory://thinking/workflows/2026/03/w",
            env.cfg.decay_grace_days_workflows(),
        ),
        ("memory://notes/n", env.cfg.decay_grace_days_default()),
    ];
    for (path, grace_days) in cases {
        // At exactly grace_days, weight must be 1.0 (boundary inclusive)
        let at_boundary = rfc3339_days_ago(grace_days);
        let weight = memory::compute_decay_weight(path, Some(&at_boundary), &env.cfg, None);
        assert!(
            (weight - 1.0).abs() <= 0.0001,
            "at grace boundary: expected 1.0 ± 0.0001, got {weight} for path={path}"
        );

        // At grace_days + 2 the power-law effective_age is 2, giving 2^(-0.5) ≈ 0.707.
        // (+1 is insufficient because effective_age=1 and 1^(-0.5) = 1.0 is a
        // mathematical identity, not a grace-window inclusion.)
        let past_boundary = rfc3339_days_ago(grace_days + 2);
        let weight_past =
            memory::compute_decay_weight(path, Some(&past_boundary), &env.cfg, None);
        assert!(
            weight_past < 1.0,
            "past grace boundary: expected < 1.0, got {weight_past} for path={path}"
        );
    }
}

// T-GRAPH-DECAY-001.4: RTM NFR-7.5.3
// REWRITE: derives expected value from config instead of hardcoding 0.1826
#[test]
fn power_law_weight_at_one_half_life_matches_formula() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let half_life = env.cfg.decay_half_life_days() as f64;
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);
    let expected = half_life.powf(-0.5); // power-law: (age_ratio)^(-0.5) where age_ratio = half_life

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, expected, 0.0001);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
// KEEP: at exactly one half-life, exponential decay must give 0.5
#[test]
fn compute_decay_weight_exponential_at_half_life_gives_point_five() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("exponential"))]);
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 0.5, 0.0001);
}

// T6 REMOVED: weak divergence assertion (abs > 0.01) redundant with T4+T5;
// mid-body env mutation was fragile

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
// REWRITE T7a: whitespace env value should trim and fall back to power-law in config
#[test]
fn whitespace_decay_function_falls_back_to_power_law_config_value() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();
    let summary = config_summary(&env);

    assert_eq!(env.cfg.decay_function(), "power-law");
    assert!(
        summary.contains("Function: power-law"),
        "config summary should report power-law fallback, got: {summary}"
    );
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
// REWRITE T7b: whitespace fallback actually routes through power-law formula
#[test]
fn whitespace_decay_function_produces_power_law_weight() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();
    let half_life = env.cfg.decay_half_life_days() as f64;
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);
    let expected = half_life.powf(-0.5);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, expected, 0.0001);
}
