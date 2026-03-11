#[path = "config_decay/support.rs"]
mod support;

// T-12.5: RTM NFR-1
use maenifold_core::memory;
use support::{
    acquire_test_lock, apply_decay_env, assert_approx_eq, clear_decay_env, config_summary,
    isolated_env, rfc3339_days_ago,
};

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_sequential_path_uses_config_grace_period() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_sequential());

    let weight = memory::compute_decay_weight(
        "memory://thinking/sequential/2026/03/session-123",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_workflow_path_uses_config_grace_period() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_workflows());

    let weight = memory::compute_decay_weight(
        "memory://thinking/workflows/2026/03/workflow-abc",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_default_path_uses_config_grace_period() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_default());

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.4: RTM NFR-7.5.3
#[test]
fn compute_decay_weight_uses_config_half_life() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 0.1826, 0.01);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn compute_decay_weight_exponential_function_applies_correctly() {
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

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn compute_decay_weight_decay_function_selection_changes_result_curve() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let power_law = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    // SAFETY: all tests in this module hold the shared test mutex for their full duration,
    // so process-wide environment mutations are serialized.
    unsafe { std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "exponential") };

    let exponential = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert!(
        (power_law - exponential).abs() > 0.01,
        "power-law and exponential decay should diverge: power_law={power_law}, exponential={exponential}"
    );
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn empty_decay_function_falls_back_to_power_law() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();
    let summary = config_summary(&env);
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_eq!(env.cfg.decay_function(), "power-law");
    assert!(summary.contains("Function: power-law"));
    assert_approx_eq(weight, 0.1826, 0.01);
}
