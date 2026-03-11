// T-12.7: RTM NFR-1
#[path = "decay_benchmark/support.rs"]
mod decay_benchmark_support;

use decay_benchmark_support::{
    TEST_FOLDER, TEST_MUTEX, assert_approx_eq, create_test_file_with_age, decay_weight_for_uri,
    isolated_env, read_memory_markdown, sync_graph,
};

// T-12.7: RTM NFR-1
#[test]
fn benchmark_decay_weight_distribution_produces_meaningful_separation() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let age_checkpoints = [1_i64, 7, 14, 30, 60, 90, 120, 180, 365];
    let mut weights = Vec::new();

    for days_ago in age_checkpoints {
        let uri = create_test_file_with_age(
            &env.cfg,
            &format!("{TEST_FOLDER}/age-{days_ago}d.md"),
            &format!("Test File ({days_ago} days old)"),
            "Identical content about [[decay-benchmark]] and [[weight-distribution]] for testing. This sentence ensures identical semantic and keyword matching across all files.",
            days_ago,
        );
        weights.push((days_ago, uri));
    }

    sync_graph(&env);

    let weights = weights
        .into_iter()
        .map(|(age, uri)| (age, decay_weight_for_uri(&env.cfg, &uri)))
        .collect::<Vec<_>>();

    let fresh_weight = weights
        .iter()
        .find(|(age, _)| *age == 1)
        .map(|(_, weight)| *weight)
        .expect("1 day weight");
    let month_old_weight = weights
        .iter()
        .find(|(age, _)| *age == 30)
        .map(|(_, weight)| *weight)
        .expect("30 day weight");
    let quarter_old_weight = weights
        .iter()
        .find(|(age, _)| *age == 90)
        .map(|(_, weight)| *weight)
        .expect("90 day weight");
    let year_old_weight = weights
        .iter()
        .find(|(age, _)| *age == 365)
        .map(|(_, weight)| *weight)
        .expect("365 day weight");

    assert!(
        fresh_weight >= 0.95,
        "1-day-old content should retain near-full weight, got {fresh_weight}"
    );
    assert!(
        month_old_weight < fresh_weight,
        "30-day-old content should weigh less than fresh content: fresh={fresh_weight}, month={month_old_weight}"
    );
    assert!(
        month_old_weight > 0.3,
        "30-day-old content should remain visible instead of being buried: {month_old_weight}"
    );
    assert!(
        quarter_old_weight < month_old_weight,
        "90-day-old content should decay further than 30-day-old content: month={month_old_weight}, quarter={quarter_old_weight}"
    );
    assert!(
        year_old_weight < 0.2,
        "365-day-old content should be heavily suppressed: {year_old_weight}"
    );
    assert!(
        year_old_weight > 0.0,
        "365-day-old content should asymptotically approach zero without reaching it: {year_old_weight}"
    );

    let suppression_ratio = fresh_weight / year_old_weight;
    assert!(
        suppression_ratio > 5.0,
        "fresh content should be at least 5x heavier than year-old content, got {suppression_ratio:.2}x"
    );
}

// T-12.7: RTM NFR-1
#[test]
fn benchmark_access_boosting_rescues_actively_used_old_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let active_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/active-old-doc.md"),
        "Architecture: Service Mesh Pattern",
        "The [[service-mesh]] pattern using [[Istio]] for [[microservices]] communication. Sidecar proxies handle routing, load balancing, and observability.",
        60,
    );
    let stale_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/stale-old-doc.md"),
        "Architecture: Monolith Pattern",
        "The [[monolith]] pattern for [[microservices]] migration. Strangler fig approach with gradual decomposition of the legacy system.",
        60,
    );

    sync_graph(&env);

    let active_weight_before = decay_weight_for_uri(&env.cfg, &active_uri);
    let stale_weight_before = decay_weight_for_uri(&env.cfg, &stale_uri);

    assert_approx_eq(active_weight_before, stale_weight_before, 0.01);

    let read_result = read_memory_markdown(&env, &active_uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory should succeed: {read_result}"
    );

    let active_weight_after = decay_weight_for_uri(&env.cfg, &active_uri);
    let stale_weight_after = decay_weight_for_uri(&env.cfg, &stale_uri);
    let boost_ratio = active_weight_after / stale_weight_after;

    assert!(
        active_weight_after >= 0.95,
        "recently read old content should regain near-full weight: {active_weight_after}"
    );
    assert!(
        stale_weight_after < 0.5,
        "untouched old content should remain decayed: {stale_weight_after}"
    );
    assert!(
        boost_ratio > 2.0,
        "access-boosted content should weigh at least 2x more than equally old stale content, got {boost_ratio:.2}x"
    );
}

// T-12.7: RTM NFR-1
#[test]
fn benchmark_tiered_decay_episodic_decays_faster_than_semantic() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let test_age_days = 10_i64;
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

    assert!(
        episodic_weight < semantic_weight,
        "episodic memory should decay faster than semantic memory at the same age: episodic={episodic_weight}, semantic={semantic_weight}"
    );
    assert_approx_eq(semantic_weight, 1.0, 0.001);
    assert!(
        episodic_weight < 1.0,
        "episodic memory should already be decaying once it passes its shorter grace period: {episodic_weight}"
    );
}
