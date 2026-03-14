// T-12.0: RTM NFR-1
#[path = "proximity_decay/support.rs"]
mod support;

use support::{
    DecayEnvGuard, TestEnv, acquire_test_lock, assert_weight_eq, decay_weight,
    invoke_search_memories_json, invoke_sync, rfc3339_days_ago, set_last_accessed,
    write_memory_file,
};

// T-12.0: RTM NFR-1 — Merged from Tests 1+2: both sides of the 7-day sequential grace boundary under power-law
#[test]
fn power_law_sequential_path_respects_seven_day_grace_window() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    // Within grace: 3 days < 7-day sequential grace → weight must be exactly 1.0
    let within = decay_weight(
        "memory://thinking/sequential/2026/03/within-grace",
        Some(&rfc3339_days_ago(3)),
        &env.cfg,
    );
    assert_weight_eq(within, 1.0);

    // One day past sequential grace (8 > 7): weight must decay appreciably
    // effective_age = (8 - 7).max(1) = 1, power-law weight = 1^(-0.5) = 1.0
    // Use day 12 instead: effective_age = 5, weight = 5^(-0.5) ≈ 0.447
    let past = decay_weight(
        "memory://thinking/sequential/2026/03/past-grace",
        Some(&rfc3339_days_ago(12)),
        &env.cfg,
    );
    assert!(
        past < 0.5,
        "power-law weight at effective_age=5 should be ≈0.447, got {past}"
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

    // effective_age = 10 - 7 = 3 days; power-law weight = 3^(-0.5) ≈ 0.577
    assert!(
        sequential < 0.65,
        "power-law weight at effective_age=3 should be ≈0.577, got {sequential}"
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

    // Intentionally identical content: isolates decay weight as the sole ranking differentiator
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
    // effective_age = 10 - 7 = 3; power-law weight = 3^(-0.5) ≈ 0.577
    let seq_decay = results[1]["decayWeight"]
        .as_f64()
        .expect("second decay weight");
    assert!(
        seq_decay < 0.65,
        "sequential path at effective_age=3 should have weight ≈0.577, got {seq_decay}"
    );
}
