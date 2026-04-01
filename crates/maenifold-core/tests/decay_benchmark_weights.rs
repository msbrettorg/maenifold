// T-12.7: RTM NFR-1
#[path = "decay_benchmark/support.rs"]
mod decay_benchmark_support;

use decay_benchmark_support::{
    DecayEnvGuard, TEST_FOLDER, TEST_MUTEX, assert_approx_eq, create_test_file_with_age,
    decay_weight_for_uri, expected_power_law_weight, isolated_env, read_memory_markdown,
    sync_graph,
};

// T-12.7: RTM NFR-1
#[test]
fn benchmark_decay_weight_distribution_produces_meaningful_separation() {
    let _guard = TEST_MUTEX.lock();
    // Pin decay config to power-law with known grace/half-life so tests are
    // hermetic regardless of developer shell environment (Rec 1).
    let _decay_env = DecayEnvGuard::pin_defaults();
    let env = isolated_env();

    // NFR-1 policy: power-law decay with 28-day default grace period.
    // Formula: weight = max(1, age - 28)^(-0.5); weight = 1.0 during grace.
    let grace_days = env.cfg.decay_grace_days_default();

    let age_checkpoints = [1_i64, 7, 14, 30, 60, 90, 120, 180, 365];
    let mut uris = Vec::new();

    for days_ago in age_checkpoints {
        let uri = create_test_file_with_age(
            &env.cfg,
            &format!("{TEST_FOLDER}/age-{days_ago}d.md"),
            &format!("Test File ({days_ago} days old)"),
            "Identical content about [[decay-benchmark]] and [[weight-distribution]] for testing. This sentence ensures identical semantic and keyword matching across all files.",
            days_ago,
        );
        uris.push((days_ago, uri));
    }

    sync_graph(&env);

    let weights: Vec<(i64, f64)> = uris
        .into_iter()
        .map(|(age, uri)| (age, decay_weight_for_uri(&env.cfg, &uri)))
        .collect();

    let weight_for = |age: i64| -> f64 {
        weights
            .iter()
            .find(|(a, _)| *a == age)
            .map_or_else(|| panic!("no weight for age {age}"), |(_, w)| *w)
    };

    // --- Assert all nine checkpoints against formula-derived values (Rec 4) ---
    // Tolerance of 0.05 allows for ±1 day of sub-day rounding in age computation
    // while remaining tight enough to catch formula regressions.
    let tolerance = 0.05;
    for &age in &age_checkpoints {
        let actual = weight_for(age);
        let expected = expected_power_law_weight(age, grace_days);
        assert!(
            (actual - expected).abs() <= tolerance,
            "age {age}d: expected weight ≈{expected:.4} (power-law, grace={grace_days}d), \
             got {actual:.4} (delta={:.4}, tolerance={tolerance})",
            (actual - expected).abs()
        );
    }

    // --- Monotonic decay: each successive age must produce equal or lower weight ---
    for pair in weights.windows(2) {
        if let [(age_a, w_a), (age_b, w_b)] = pair {
            assert!(
                w_a >= w_b,
                "decay must be monotonically non-increasing: \
                 age {age_a}d ({w_a:.4}) should be ≥ age {age_b}d ({w_b:.4})"
            );
        }
    }

    // --- Minimum-gap assertions prevent a nearly-flat curve from passing (Rec 2) ---

    // NFR-1: fresh → 30-day gap must be meaningful (actual ≈ 0.293)
    let fresh = weight_for(1);
    let month = weight_for(30);
    assert!(
        fresh - month > 0.15,
        "gap between fresh and 30-day should exceed 0.15 to confirm meaningful decay: \
         fresh={fresh:.4}, 30d={month:.4}, gap={:.4}",
        fresh - month
    );

    // NFR-1: 30-day → 90-day gap confirms continued steep decay (actual ≈ 0.580)
    let quarter = weight_for(90);
    assert!(
        month - quarter > 0.30,
        "gap between 30-day and 90-day should exceed 0.30: \
         30d={month:.4}, 90d={quarter:.4}, gap={:.4}",
        month - quarter
    );

    // NFR-1: suppression ratio — fresh vs year-old content.
    // Conservative bound: actual ≈ 18.3× for power-law; threshold 10× catches
    // curve-flattening regressions while leaving room for minor tuning.
    let year = weight_for(365);
    let suppression_ratio = fresh / year;
    assert!(
        suppression_ratio > 10.0,
        "fresh content should be at least 10× heavier than year-old content, \
         got {suppression_ratio:.2}× (fresh={fresh:.4}, year={year:.4})"
    );
}

// T-12.7: RTM NFR-1
#[test]
fn benchmark_access_boosting_rescues_actively_used_old_content() {
    let _guard = TEST_MUTEX.lock();
    // Pin decay config for hermetic test execution (Rec 1).
    let _decay_env = DecayEnvGuard::pin_defaults();
    let env = isolated_env();

    // 60-day age chosen to be clearly past the 28-day default grace period,
    // producing a decayed baseline weight (≈0.18 under power-law) while leaving
    // room for the access boost to produce a dramatic recovery to ~1.0.
    let test_age_days = 60_i64;
    let grace_days = env.cfg.decay_grace_days_default();

    let active_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/active-old-doc.md"),
        "Architecture: Service Mesh Pattern",
        "The [[service-mesh]] pattern using [[Istio]] for [[microservices]] communication. Sidecar proxies handle routing, load balancing, and observability.",
        test_age_days,
    );
    let stale_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/stale-old-doc.md"),
        "Architecture: Monolith Pattern",
        "The [[monolith]] pattern for [[microservices]] migration. Strangler fig approach with gradual decomposition of the legacy system.",
        test_age_days,
    );

    sync_graph(&env);

    // Pre-condition: both files should have equal decay weight before any access.
    // Tolerance of 0.01 at weight ≈0.18 represents ~5.7% relative difference,
    // accounting for sub-second timestamp variation in file creation.
    let active_weight_before = decay_weight_for_uri(&env.cfg, &active_uri);
    let stale_weight_before = decay_weight_for_uri(&env.cfg, &stale_uri);
    assert_approx_eq(active_weight_before, stale_weight_before, 0.01);

    // Verify pre-boost weight matches formula expectation (power-law, 60d age)
    let expected_pre_boost = expected_power_law_weight(test_age_days, grace_days);
    assert_approx_eq(
        active_weight_before,
        expected_pre_boost,
        0.05, // tolerance for ±1 day rounding
    );

    // Access boost: read_memory updates last_accessed → weight resets to ~1.0
    let read_result = read_memory_markdown(&env, &active_uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory should succeed: {read_result}"
    );

    let active_weight_after = decay_weight_for_uri(&env.cfg, &active_uri);
    let stale_weight_after = decay_weight_for_uri(&env.cfg, &stale_uri);
    let boost_ratio = active_weight_after / stale_weight_after;

    // NFR-1 policy: accessed content should regain near-full weight.
    // Threshold 0.95 allows for sub-second timing variance where last_accessed
    // might not land exactly at now.
    assert!(
        active_weight_after >= 0.95,
        "recently read old content should regain near-full weight: {active_weight_after:.4}"
    );

    // Tightened from < 0.5 → < 0.25 (Rec 3). Actual stale weight ≈ 0.18;
    // threshold 0.25 catches cache-invalidation bugs that inflate stale weight
    // to 0.3–0.4 while leaving margin above the actual value.
    assert!(
        stale_weight_after < 0.25,
        "untouched old content should remain heavily decayed: {stale_weight_after:.4} \
         (expected ≈{expected_pre_boost:.4})"
    );

    // Tightened from > 2.0 → > 4.0 (Rec 3). Actual ratio ≈ 5.65×;
    // threshold 4.0× catches partial-boost regressions (e.g., boost applies but
    // doesn't fully reset last_accessed) while giving margin for minor drift.
    assert!(
        boost_ratio > 4.0,
        "access-boosted content should weigh at least 4× more than equally old \
         stale content, got {boost_ratio:.2}× \
         (active={active_weight_after:.4}, stale={stale_weight_after:.4})"
    );
}

// T-12.7: RTM NFR-1
// Renamed from benchmark_tiered_decay_episodic_decays_faster_than_semantic (Rec 8)
// to remove implication of complete tiered coverage — this tests only the
// episodic vs semantic tiers. See benchmark_workflow_decays_between_episodic_and_semantic
// for the workflow tier.
#[test]
fn benchmark_episodic_decays_faster_than_semantic() {
    let _guard = TEST_MUTEX.lock();
    // Pin decay config for hermetic test execution (Rec 1).
    let _decay_env = DecayEnvGuard::pin_defaults();
    let env = isolated_env();

    let test_age_days = 10_i64;
    let grace_sequential = env.cfg.decay_grace_days_sequential();
    let grace_default = env.cfg.decay_grace_days_default();

    // Pre-condition guard (Rec 5): assert the test age falls in the window
    // where episodic content has exited its grace period but semantic content
    // has not. If config changes break this invariant, the test fails loudly
    // rather than silently passing with both weights at 1.0.
    assert!(
        test_age_days > grace_sequential,
        "test age ({test_age_days}d) must exceed episodic grace period ({grace_sequential}d) \
         for episodic decay to be observable"
    );
    assert!(
        test_age_days < grace_default,
        "test age ({test_age_days}d) must be within semantic grace period ({grace_default}d) \
         for semantic weight to remain at 1.0"
    );

    let episodic_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/thinking/sequential/debug-session.md"),
        "Debug Session",
        "Debugging [[tiered-decay]] behavior.",
        test_age_days,
    );
    let semantic_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/knowledge-doc.md"),
        "Knowledge Document",
        "Knowledge about [[tiered-decay]] behavior.",
        test_age_days,
    );

    sync_graph(&env);

    let episodic_weight = decay_weight_for_uri(&env.cfg, &episodic_uri);
    let semantic_weight = decay_weight_for_uri(&env.cfg, &semantic_uri);

    // Verify against formula expectations
    let expected_episodic = expected_power_law_weight(test_age_days, grace_sequential);
    let expected_semantic = expected_power_law_weight(test_age_days, grace_default);

    assert_approx_eq(episodic_weight, expected_episodic, 0.05);
    // Semantic at 10d with 28d grace → exactly 1.0
    assert_approx_eq(semantic_weight, expected_semantic, 0.001);

    assert!(
        episodic_weight < semantic_weight,
        "episodic memory should decay faster than semantic memory at the same age: \
         episodic={episodic_weight:.4}, semantic={semantic_weight:.4}"
    );

    // Minimum-separation assertion (Rec 2): prevents passing with a nearly-flat
    // curve where episodic ≈ 0.9999 and semantic = 1.0.
    // Actual gap ≈ 0.42 (1.0 - 0.577); threshold 0.3 catches tier-dispatch bugs.
    assert!(
        semantic_weight - episodic_weight > 0.3,
        "tier separation must be functionally meaningful, not just directional: \
         gap={:.4} (semantic={semantic_weight:.4}, episodic={episodic_weight:.4})",
        semantic_weight - episodic_weight
    );
}

// T-12.7: RTM NFR-1
// Companion test for the workflow tier (Rec 6). The grace_period_for_path function
// distinguishes three tiers: sequential (7d), workflow (14d), and default (28d).
// This test verifies the workflow tier produces weights between episodic and semantic.
#[test]
fn benchmark_workflow_decays_between_episodic_and_semantic() {
    let _guard = TEST_MUTEX.lock();
    // Pin decay config for hermetic test execution (Rec 1).
    let _decay_env = DecayEnvGuard::pin_defaults();
    let env = isolated_env();

    // Age 20 days: past sequential grace (7d) and workflow grace (14d),
    // but within default grace (28d). This separates all three tiers.
    let test_age_days = 20_i64;
    let grace_sequential = env.cfg.decay_grace_days_sequential();
    let grace_workflows = env.cfg.decay_grace_days_workflows();
    let grace_default = env.cfg.decay_grace_days_default();

    // Pre-condition guards: age must separate all three tiers
    assert!(
        test_age_days > grace_workflows,
        "test age ({test_age_days}d) must exceed workflow grace ({grace_workflows}d)"
    );
    assert!(
        test_age_days > grace_sequential,
        "test age ({test_age_days}d) must exceed sequential grace ({grace_sequential}d)"
    );
    assert!(
        test_age_days < grace_default,
        "test age ({test_age_days}d) must be within default grace ({grace_default}d)"
    );

    let episodic_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/thinking/sequential/workflow-tier-test.md"),
        "Sequential Session",
        "Testing [[tiered-decay]] three-tier separation.",
        test_age_days,
    );
    let workflow_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/thinking/workflow/planning-session.md"),
        "Workflow Session",
        "Testing [[tiered-decay]] three-tier separation.",
        test_age_days,
    );
    let semantic_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/reference-doc.md"),
        "Reference Document",
        "Testing [[tiered-decay]] three-tier separation.",
        test_age_days,
    );

    sync_graph(&env);

    let episodic_weight = decay_weight_for_uri(&env.cfg, &episodic_uri);
    let workflow_weight = decay_weight_for_uri(&env.cfg, &workflow_uri);
    let semantic_weight = decay_weight_for_uri(&env.cfg, &semantic_uri);

    // Verify against formula expectations
    // Sequential (7d grace): effective_age = 13, weight ≈ 0.277
    let expected_episodic = expected_power_law_weight(test_age_days, grace_sequential);
    // Workflow (14d grace): effective_age = 6, weight ≈ 0.408
    let expected_workflow = expected_power_law_weight(test_age_days, grace_workflows);
    // Default (28d grace): within grace → weight = 1.0
    let expected_semantic = expected_power_law_weight(test_age_days, grace_default);

    assert_approx_eq(episodic_weight, expected_episodic, 0.05);
    assert_approx_eq(workflow_weight, expected_workflow, 0.05);
    assert_approx_eq(semantic_weight, expected_semantic, 0.001);

    // Strict ordering: episodic < workflow < semantic
    assert!(
        episodic_weight < workflow_weight,
        "episodic (sequential grace={grace_sequential}d) should decay more than \
         workflow (grace={grace_workflows}d): episodic={episodic_weight:.4}, \
         workflow={workflow_weight:.4}"
    );
    assert!(
        workflow_weight < semantic_weight,
        "workflow (grace={grace_workflows}d) should decay more than semantic \
         (grace={grace_default}d): workflow={workflow_weight:.4}, \
         semantic={semantic_weight:.4}"
    );

    // Minimum-separation: each tier gap must be meaningful
    // Episodic→Workflow gap: actual ≈ 0.131; threshold 0.08 catches mis-dispatch
    assert!(
        workflow_weight - episodic_weight > 0.08,
        "workflow-episodic gap must be meaningful: gap={:.4} \
         (workflow={workflow_weight:.4}, episodic={episodic_weight:.4})",
        workflow_weight - episodic_weight
    );
    // Workflow→Semantic gap: actual ≈ 0.592; threshold 0.40 catches tier collapse
    assert!(
        semantic_weight - workflow_weight > 0.40,
        "semantic-workflow gap must be meaningful: gap={:.4} \
         (semantic={semantic_weight:.4}, workflow={workflow_weight:.4})",
        semantic_weight - workflow_weight
    );
}
