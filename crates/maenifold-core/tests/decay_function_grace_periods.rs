#[path = "decay_function/support.rs"]
mod decay_function_support;

use chrono::Utc;
use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, DecayEnvGuard, SEQUENTIAL_GRACE_PERIOD_DAYS,
    TestConfigGuard, WORKFLOWS_GRACE_PERIOD_DAYS, acquire_test_lock, assert_approx_eq,
    compute_exponential_decay, compute_with_last_accessed, last_accessed_for_age_days,
};

// T-12.6: RTM NFR-1
#[test]
fn exponential_decay_sequential_thinking_uses_7_day_grace_period() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/sequential/2026/03/at-grace",
        SEQUENTIAL_GRACE_PERIOD_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/sequential/2026/03/past-grace",
        SEQUENTIAL_GRACE_PERIOD_DAYS + 1,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "sequential content should decay after day 7"
    );
}

#[test]
fn exponential_decay_workflows_uses_14_day_grace_period() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/workflow/2026/03/at-grace",
        WORKFLOWS_GRACE_PERIOD_DAYS,
        WORKFLOWS_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/workflow/2026/03/past-grace",
        WORKFLOWS_GRACE_PERIOD_DAYS + 1,
        WORKFLOWS_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "workflow content should decay after day 14"
    );
}

#[test]
fn exponential_decay_default_memory_uses_14_day_grace_period() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://projects/default-at-grace",
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://projects/default-past-grace",
        DEFAULT_GRACE_PERIOD_DAYS + 1,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "default content should decay after day 14"
    );
}

#[test]
fn exponential_decay_tiered_grace_periods_sequential_decays_faster() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, 10);
    let _env = DecayEnvGuard::set(
        "exponential",
        DEFAULT_HALF_LIFE_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
        WORKFLOWS_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    let sequential = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/sequential/2026/03/note",
        &last_accessed,
        None,
    );
    let workflow = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/workflow/2026/03/note",
        &last_accessed,
        None,
    );
    let default = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/default-note",
        &last_accessed,
        None,
    );

    assert!(sequential < workflow);
    assert!(sequential < default);
    assert_approx_eq(workflow, default, 1e-9);
}
