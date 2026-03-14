#[path = "decay_calculator/support.rs"]
mod decay_calculator_support;

use decay_calculator_support::{
    DecayEnvGuard, TEST_MUTEX, assert_approx_eq, isolated_env, weight_for,
};

// T-12.7: RTM NFR-1

// T-7.5.2: RTM NFR-7.5.2
/// Empty path routes to default grace period (28 days).
/// Verifies both at-boundary (day 28 == 1.0) and post-grace (day 29 < 1.0)
/// to prove routing is correct and not masked by a longer grace period.
#[test]
fn empty_path_uses_default_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    // T-7.5.2: At the default grace boundary (day 28), weight must be 1.0
    let at_grace = weight_for(&env.cfg, "", Some(28), None);
    assert_approx_eq(at_grace, 1.0, 1e-9);

    // T-7.5.2: One day past default grace (day 29), weight must drop below 1.0
    let post_grace = weight_for(&env.cfg, "", Some(29), None);
    assert!(
        post_grace < 1.0,
        "expected decay at day 29 (one past default grace), got {post_grace}"
    );
}

// T-7.5.1: RTM NFR-7.5.1
/// Windows-style backslash path containing `thinking/sequential` routes to
/// sequential grace period (7 days). Verifies at-boundary, post-grace, and
/// comparative behavior to prove correct routing (not masked by a longer grace).
#[test]
fn windows_backslash_sequential_path_uses_sequential_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let windows_path = r"C:\Users\user\maenifold\memory\thinking\sequential\session.md";

    // T-7.5.1: At the sequential grace boundary (day 7), weight must be 1.0
    let at_grace = weight_for(&env.cfg, windows_path, Some(7), None);
    assert_approx_eq(at_grace, 1.0, 1e-9);

    // T-7.5.1: One day past sequential grace (day 8), weight MUST be < 1.0.
    // This is the decisive assertion: if the path were misrouted to default
    // grace (28d), day 8 would still return 1.0.
    let post_grace = weight_for(&env.cfg, windows_path, Some(8), None);
    assert!(
        post_grace < 1.0,
        "expected decay at day 8 (one past sequential grace), got {post_grace}"
    );

    // T-7.5.1: Comparative — at day 10, sequential path must decay faster than
    // a regular (default grace) path. Merged from former test 5.
    let sequential_at_10 = weight_for(&env.cfg, windows_path, Some(10), None);
    let regular_at_10 = weight_for(&env.cfg, "memory://notes/reference-note", Some(10), None);
    assert!(
        sequential_at_10 < regular_at_10,
        "expected sequential={sequential_at_10} to decay faster than regular={regular_at_10} at day 10"
    );
}
