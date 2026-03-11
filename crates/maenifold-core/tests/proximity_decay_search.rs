// T-12.0: RTM NFR-1
#[path = "proximity_decay/support.rs"]
mod support;

use support::{
    DecayEnvGuard, TestEnv, acquire_test_lock, assert_weight_eq, decay_weight,
    invoke_search_memories_json, invoke_sync, rfc3339_days_ago, set_last_accessed,
    write_memory_file,
};

// T-12.0: RTM NFR-1
#[test]
fn compute_decay_weight_is_one_within_grace_period() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();
    let last_accessed = rfc3339_days_ago(3);

    let weight = decay_weight(
        "memory://thinking/sequential/2026/03/within-grace",
        Some(&last_accessed),
        &env.cfg,
    );

    assert_weight_eq(weight, 1.0);
}

// T-12.0: RTM NFR-1
#[test]
fn compute_decay_weight_drops_below_one_after_grace_period() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();
    let last_accessed = rfc3339_days_ago(12);

    let weight = decay_weight(
        "memory://thinking/sequential/2026/03/beyond-grace",
        Some(&last_accessed),
        &env.cfg,
    );

    assert!(
        weight < 1.0,
        "weight should decay after grace period: {weight}"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn compute_decay_weight_uses_path_specific_grace_windows() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();
    let last_accessed = rfc3339_days_ago(10);

    let sequential = decay_weight(
        "memory://thinking/sequential/2026/03/sequential-note",
        Some(&last_accessed),
        &env.cfg,
    );
    let workflow = decay_weight(
        "memory://thinking/workflow/2026/03/workflow-note",
        Some(&last_accessed),
        &env.cfg,
    );
    let standard = decay_weight(
        "memory://projects/default-note",
        Some(&last_accessed),
        &env.cfg,
    );

    assert!(
        sequential < 1.0,
        "sequential notes should decay after the 7-day grace period: {sequential}"
    );
    assert_weight_eq(workflow, 1.0);
    assert_weight_eq(standard, 1.0);
}

// T-12.0: RTM NFR-1
#[test]
fn search_memories_orders_results_by_decay_weighted_score() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    let sequential_uri = write_memory_file(
        &env.cfg,
        "thinking/sequential/2026/03/sequential-ranked.md",
        "# Ranked\n\nThis note repeats ranking-target for search ordering.\n",
    );
    let default_uri = write_memory_file(
        &env.cfg,
        "projects/default-ranked.md",
        "# Ranked\n\nThis note repeats ranking-target for search ordering.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let ten_days_ago = rfc3339_days_ago(10);
    set_last_accessed(&env.cfg, &sequential_uri, &ten_days_ago);
    set_last_accessed(&env.cfg, &default_uri, &ten_days_ago);

    let response = invoke_search_memories_json("ranking-target");
    assert_eq!(
        response["success"], true,
        "search should succeed: {response}"
    );
    let results = response["data"]["results"]
        .as_array()
        .expect("results array");

    assert!(
        results.len() >= 2,
        "expected at least two ranked results: {response}"
    );
    assert_eq!(
        results[0]["path"].as_str(),
        Some(default_uri.as_str()),
        "default memory should outrank sequential memory after decay: {response}"
    );
    assert_eq!(
        results[1]["path"].as_str(),
        Some(sequential_uri.as_str()),
        "sequential memory should rank below the non-decayed result: {response}"
    );
    assert!(
        results[0]["fusedScore"].as_f64().expect("top fused score")
            > results[1]["fusedScore"]
                .as_f64()
                .expect("second fused score"),
        "decay should reduce the sequential fused score: {response}"
    );
    assert_weight_eq(
        results[0]["decayWeight"]
            .as_f64()
            .expect("top decay weight"),
        1.0,
    );
    assert!(
        results[1]["decayWeight"]
            .as_f64()
            .expect("second decay weight")
            < 1.0,
        "sequential result should carry a decayed weight: {response}"
    );
}
