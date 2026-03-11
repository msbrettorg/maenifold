#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1

// T-7.5.2: RTM NFR-7.5.2
#[test]
fn empty_path_uses_default_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "", Some(28), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1: RTM NFR-7.5.1
#[test]
fn windows_backslash_sequential_path_uses_sequential_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let windows_path = r"C:\\Users\\user\\maenifold\\memory\\thinking\\sequential\\session.md";
    let weight = weight_for(&env.cfg, windows_path, Some(7), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1a: RTM NFR-7.5.1
#[test]
fn thinking_workflow_path_uses_workflow_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/workflow-note",
        Some(14),
        None,
    );

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.2: RTM NFR-7.5.2
#[test]
fn regular_path_uses_default_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "memory://notes/my-note", Some(28), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1: RTM NFR-7.5.1
#[test]
fn sequential_path_has_shorter_grace_than_default_path() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let sequential = weight_for(
        &env.cfg,
        "memory://thinking/sequential/2026/03/session",
        Some(10),
        None,
    );
    let regular = weight_for(&env.cfg, "memory://notes/reference-note", Some(10), None);

    assert!(
        sequential < regular,
        "expected sequential={sequential} to decay faster than regular={regular}"
    );
    assert_approx_eq(regular, 1.0, 1e-9);
}
