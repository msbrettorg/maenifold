#[path = "config_decay/support.rs"]
mod support;

// T-12.5: RTM NFR-1
use support::{acquire_test_lock, apply_decay_env, clear_decay_env, config_summary, isolated_env};

// T-GRAPH-DECAY-001.2: RTM NFR-7.5.1
#[test]
fn decay_grace_days_sequential_default_is_seven() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
}

// T-GRAPH-DECAY-001.2a: RTM NFR-7.5.1a
#[test]
fn decay_grace_days_workflows_default_is_fourteen() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_workflows(), 14);
}

// T-GRAPH-DECAY-001.3: RTM NFR-7.5.2
#[test]
fn decay_grace_days_default_default_is_twenty_eight() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_default(), 28);
}

// T-GRAPH-DECAY-001.4: RTM NFR-7.5.3
#[test]
fn decay_half_life_days_default_is_thirty() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_half_life_days(), 30);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn decay_function_default_is_power_law() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_function(), "power-law");
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

    let summary = config_summary(&env);

    assert!(summary.contains("Grace Days (Sequential): 9"));
    assert!(summary.contains("Grace Days (Workflows): 18"));
    assert!(summary.contains("Grace Days (Default): 33"));
    assert!(summary.contains("Half-Life Days: 45"));
    assert!(summary.contains("Function: exponential"));
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
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
    let summary = config_summary(&env);

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
    assert_eq!(env.cfg.decay_grace_days_workflows(), 14);
    assert_eq!(env.cfg.decay_grace_days_default(), 28);
    assert_eq!(env.cfg.decay_half_life_days(), 30);
    assert!(summary.contains("Grace Days (Sequential): 7"));
    assert!(summary.contains("Grace Days (Workflows): 14"));
    assert!(summary.contains("Grace Days (Default): 28"));
    assert!(summary.contains("Half-Life Days: 30"));
}
