#[path = "config_decay/support.rs"]
mod support;

// T-12.5: RTM NFR-1
use support::{acquire_test_lock, apply_decay_env, clear_decay_env, config_summary, isolated_env};

// T-GRAPH-DECAY-001.2 · T-GRAPH-DECAY-001.2a · T-GRAPH-DECAY-001.3
// T-GRAPH-DECAY-001.4 · T-GRAPH-DECAY-001.6: RTM NFR-7.5.1 · NFR-7.5.2 · NFR-7.5.3 · NFR-7.5.5
// Merged from T1–T5: five isomorphic default-value tests collapsed into one table-driven test.
#[test]
fn decay_config_accessor_defaults_match_spec() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(
        env.cfg.decay_grace_days_sequential(),
        7,
        "sequential grace days"
    );
    assert_eq!(
        env.cfg.decay_grace_days_workflows(),
        14,
        "workflow grace days"
    );
    assert_eq!(env.cfg.decay_grace_days_default(), 28, "default grace days");
    assert_eq!(env.cfg.decay_half_life_days(), 30, "half-life days");
    assert_eq!(env.cfg.decay_function(), "power-law", "function");
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn get_config_summary_includes_decay_config() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    let summary = config_summary(&env);

    assert!(summary.contains("Decay Config:"));
    assert!(summary.contains("Grace Days (Sequential):"));
    assert!(summary.contains("Grace Days (Workflows):"));
    assert!(summary.contains("Grace Days (Default):"));
    assert!(summary.contains("Half-Life Days:"));
    assert!(summary.contains("Function:"));
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn get_config_summary_shows_actual_values() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[
        ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("9")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", Some("18")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", Some("33")),
        ("MAENIFOLD_DECAY_HALF_LIFE_DAYS", Some("45")),
        ("MAENIFOLD_DECAY_FUNCTION", Some("exponential")),
    ]);
    let env = isolated_env();

    // Direct accessor assertions close the indirect coverage gap (Rec #3).
    assert_eq!(
        env.cfg.decay_grace_days_sequential(),
        9,
        "sequential accessor"
    );
    assert_eq!(
        env.cfg.decay_grace_days_workflows(),
        18,
        "workflow accessor"
    );
    assert_eq!(env.cfg.decay_grace_days_default(), 33, "default accessor");
    assert_eq!(env.cfg.decay_half_life_days(), 45, "half-life accessor");
    assert_eq!(env.cfg.decay_function(), "exponential", "function accessor");

    let summary = config_summary(&env);

    assert!(summary.contains("Grace Days (Sequential): 9"));
    assert!(summary.contains("Grace Days (Workflows): 18"));
    assert!(summary.contains("Grace Days (Default): 33"));
    assert!(summary.contains("Half-Life Days: 45"));
    assert!(summary.contains("Function: exponential"));
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
// Rewritten from T8: tightened to focus on integer fallback with failure messages.
#[test]
fn invalid_integer_decay_env_vars_fall_back_to_defaults() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[
        ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", Some("invalid")),
        ("MAENIFOLD_DECAY_HALF_LIFE_DAYS", Some("invalid")),
    ]);
    let env = isolated_env();

    assert_eq!(
        env.cfg.decay_grace_days_sequential(),
        7,
        "sequential grace days"
    );
    assert_eq!(
        env.cfg.decay_grace_days_workflows(),
        14,
        "workflow grace days"
    );
    assert_eq!(env.cfg.decay_grace_days_default(), 28, "default grace days");
    assert_eq!(env.cfg.decay_half_life_days(), 30, "half-life days");
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
// New: documents the env_string pass-through contract — unrecognized function names
// are NOT defaulted; downstream code must handle them.
#[test]
fn unrecognized_decay_function_string_is_passed_through_not_defaulted() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("bogus-function"))]);
    let env = isolated_env();

    // env_string does NOT fall back to "power-law" for unrecognized strings.
    // The caller (decay scoring) is responsible for handling unrecognized values.
    assert_eq!(env.cfg.decay_function(), "bogus-function");
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
// New: verifies that whitespace-only MAENIFOLD_DECAY_FUNCTION falls back to default
// because env_string filters empty-after-trim.
#[test]
fn whitespace_only_decay_function_falls_back_to_default() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();

    // env_string trims whitespace and filters empty strings → falls back to default.
    assert_eq!(env.cfg.decay_function(), "power-law");
}
