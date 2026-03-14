// T-12.6: RTM NFR-1, NFR-2
//
// Tests for the grace-period routing logic of the exponential decay function.
// Each path tier (sequential=7d, workflow=14d, default=28d) gets its own grace
// period via `grace_period_for_path`. Tests use differentiated grace periods to
// verify routing isolation — not the flat-all-same `compute_exponential_decay`
// helper that masks routing bugs.

#[path = "decay_function/support.rs"]
mod decay_function_support;

use chrono::Utc;
use decay_function_support::{
    DEFAULT_GRACE_PERIOD_DAYS, DEFAULT_HALF_LIFE_DAYS, DecayEnvGuard, SEQUENTIAL_GRACE_PERIOD_DAYS,
    TestConfigGuard, WORKFLOWS_GRACE_PERIOD_DAYS, acquire_test_lock, compute_with_last_accessed,
    last_accessed_for_age_days,
};

// T-12.6: RTM NFR-1, NFR-2
/// Verifies that each path tier routes to its correct grace period constant
/// by setting differentiated grace periods (7/14/28) and checking at-boundary
/// (score == 1.0) and one-past-boundary (score < 1.0) for each tier.
///
/// Also verifies routing isolation: at age=8, sequential (grace=7) must be
/// decaying while default (grace=28) must still be in grace.
///
/// Merged from former Tests 1-3 which each set ALL grace env vars to the same
/// value, making the path string irrelevant to the assertion.
#[test]
fn grace_period_boundary_gate_per_tier() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let now = Utc::now();

    // T-12.6: RTM NFR-1 — Set production-correct differentiated grace periods
    let _decay = DecayEnvGuard::set(
        "exponential",
        DEFAULT_HALF_LIFE_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS, // 7
        WORKFLOWS_GRACE_PERIOD_DAYS,  // 14
        DEFAULT_GRACE_PERIOD_DAYS,    // 28
    );

    let tiers: &[(&str, i64)] = &[
        (
            "memory://thinking/sequential/2026/03/boundary-test",
            SEQUENTIAL_GRACE_PERIOD_DAYS,
        ),
        (
            "memory://thinking/workflow/2026/03/boundary-test",
            WORKFLOWS_GRACE_PERIOD_DAYS,
        ),
        (
            "memory://projects/boundary-test",
            DEFAULT_GRACE_PERIOD_DAYS,
        ),
    ];

    for &(path, grace_days) in tiers {
        // At grace boundary: score must be exactly 1.0
        let at_grace_accessed = last_accessed_for_age_days(now, grace_days);
        let at_grace = compute_with_last_accessed(&env.cfg, path, &at_grace_accessed, None);
        assert_eq!(
            at_grace, 1.0,
            "{path} at age={grace_days} (grace boundary) should be 1.0"
        );

        // One day past grace: score must drop below 1.0
        let past_grace_accessed = last_accessed_for_age_days(now, grace_days + 1);
        let past_grace = compute_with_last_accessed(&env.cfg, path, &past_grace_accessed, None);
        assert!(
            past_grace < 1.0,
            "{path} at age={} (one past grace) should be < 1.0, got {past_grace}",
            grace_days + 1
        );
    }

    // T-12.6: RTM NFR-1 — Routing isolation: sequential at age=8 must decay
    // while default at age=8 must NOT (since default grace=28, age=8 is within grace)
    let age_8_accessed = last_accessed_for_age_days(now, 8);
    let seq_at_8 = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/sequential/2026/03/routing-test",
        &age_8_accessed,
        None,
    );
    let default_at_8 = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/routing-test",
        &age_8_accessed,
        None,
    );
    assert!(
        seq_at_8 < 1.0,
        "sequential at day 8 should be past grace (grace=7), got {seq_at_8}"
    );
    assert_eq!(
        default_at_8, 1.0,
        "default at day 8 should still be in grace (grace=28)"
    );
}

// T-12.6: RTM NFR-1, NFR-2
/// At the same age, sequential content decays faster than workflow/default
/// because of its shorter grace period. Verified at two age points:
///
/// - age=10: sequential is past grace (7d), workflow (14d) and default (28d)
///   are still in grace — proves the tier ordering within the grace window.
/// - age=30: all tiers are past grace — proves the full ordering in active
///   decay where workflow (16d of decay) has decayed more than default (2d).
#[test]
fn tiered_grace_periods_sequential_decays_faster_with_post_grace_ordering() {
    let _guard = acquire_test_lock();
    let env = TestConfigGuard::new();
    let now = Utc::now();

    // T-12.6: RTM NFR-1 — differentiated grace periods matching production defaults
    let _decay = DecayEnvGuard::set(
        "exponential",
        DEFAULT_HALF_LIFE_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS, // 7
        WORKFLOWS_GRACE_PERIOD_DAYS,  // 14
        DEFAULT_GRACE_PERIOD_DAYS,    // 28
    );

    // --- Block 1: age=10 — sequential past grace, others in grace ---
    let age_10_accessed = last_accessed_for_age_days(now, 10);
    let sequential = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/sequential/2026/03/note",
        &age_10_accessed,
        None,
    );
    let workflow = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/workflow/2026/03/note",
        &age_10_accessed,
        None,
    );
    let default = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/default-note",
        &age_10_accessed,
        None,
    );

    // T-12.6: RTM NFR-1 — sequential must be decaying while others are in grace
    assert!(
        sequential < 1.0,
        "sequential at day 10 should be past grace (grace=7), got {sequential}"
    );
    assert!(
        sequential < workflow,
        "sequential should decay faster than workflow at day 10"
    );
    assert!(
        sequential < default,
        "sequential should decay faster than default at day 10"
    );
    // T-12.6: RTM NFR-1 — workflow and default both in grace at day 10, must be exactly 1.0
    assert_eq!(
        workflow, 1.0,
        "workflow at day 10 should be in grace (grace=14)"
    );
    assert_eq!(
        default, 1.0,
        "default at day 10 should be in grace (grace=28)"
    );

    // --- Block 2: age=30 — all tiers past grace, verify full ordering ---
    let age_30_accessed = last_accessed_for_age_days(now, 30);
    let sequential_post = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/sequential/2026/03/note",
        &age_30_accessed,
        None,
    );
    let workflow_post = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/workflow/2026/03/note",
        &age_30_accessed,
        None,
    );
    let default_post = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/default-note",
        &age_30_accessed,
        None,
    );

    // T-12.6: RTM NFR-1 — all tiers actively decaying past their grace windows
    assert!(
        sequential_post < 1.0,
        "sequential at day 30 should be past grace (grace=7), got {sequential_post}"
    );
    assert!(
        workflow_post < 1.0,
        "workflow at day 30 should be past grace (grace=14), got {workflow_post}"
    );
    assert!(
        default_post < 1.0,
        "default at day 30 should be past grace (grace=28), got {default_post}"
    );

    // T-12.6: RTM NFR-1 — full tier ordering in active decay:
    // sequential (23d of decay) < workflow (16d of decay) < default (2d of decay)
    assert!(
        sequential_post < workflow_post,
        "sequential ({sequential_post}) should have decayed more than workflow ({workflow_post}) at day 30"
    );
    assert!(
        workflow_post < default_post,
        "workflow ({workflow_post}) should have decayed more than default ({default_post}) at day 30"
    );
}
