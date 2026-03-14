// T-12.7: RTM NFR-1
#[path = "decay_benchmark/support.rs"]
mod decay_benchmark_support;

use decay_benchmark_support::{
    TEST_FOLDER, TEST_MUTEX, create_test_file_with_age, find_document_rank, find_result,
    format_rank, isolated_env, search_memories_json, sync_graph,
};

// T-12.7: RTM NFR-1
/// Verifies that a recent decision document outranks stale research notes in
/// hybrid search, and that the decay mechanism is actively suppressing old
/// content — not just relying on text-relevance dominance.
#[test]
fn recent_decision_outranks_stale_research_in_hybrid_search() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth-research-1.md"),
        "Auth Research: OAuth2 vs SAML",
        "Research on [[authentication]] approaches. Comparing [[OAuth2]] and [[SAML]] for the [[API]]. OAuth2 is more common for REST APIs. SAML is enterprise-focused. Need to decide.",
        90,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth-research-2.md"),
        "Auth Research: JWT Considerations",
        "Investigating [[JWT]] [[token]] strategies for [[authentication]] in the [[API]]. Refresh tokens, expiry policies, and revocation challenges documented.",
        85,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth-debug-1.md"),
        "Debug: Auth Middleware Failing",
        "Debugging [[authentication]] middleware in the [[API]] layer. The bearer token validation was failing due to clock skew. Fixed by adding tolerance.",
        75,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth-debug-2.md"),
        "Debug: CORS and Auth Headers",
        "The [[API]] [[authentication]] headers were being stripped by CORS preflight. Added proper Access-Control-Allow-Headers configuration.",
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-auth-meeting.md"),
        "Meeting Notes: Auth Strategy Discussion",
        "Team discussion about [[authentication]] strategy for the [[API]]. No consensus reached. Tabled for next sprint. Action items pending.",
        50,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/auth-decision.md"),
        "DECISION: API Authentication Approach",
        "Final decision on [[authentication]] for the [[API]]: use [[OAuth2]] with PKCE flow. JWT access tokens with 15-min expiry. Refresh tokens stored server-side. This supersedes all prior research and debugging notes on auth.",
        1,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "authentication approach for the API", "Hybrid");
    let decision_index =
        find_document_rank(&search_result, "DECISION: API Authentication Approach");

    assert!(
        decision_index.is_some(),
        "decision document should appear in search results: {search_result}"
    );
    assert!(
        decision_index.expect("decision rank") < 2,
        "recent decision document should rank in the top 2 instead of being buried by stale auth notes: {search_result}"
    );

    // T-12.7: RTM NFR-1 — assert decay mechanism is active on the recent decision
    let decision_result = find_result(&search_result, "DECISION: API Authentication Approach")
        .expect("decision result in search output");
    let decision_decay = decision_result["decayWeight"]
        .as_f64()
        .expect("decision decayWeight");
    assert!(
        decision_decay >= 0.95,
        "recent decision (1 day old) should have near-full decay weight, got {decision_decay}: {search_result}"
    );

    // T-12.7: RTM NFR-1 — assert oldest stale doc is suppressed by decay
    let oldest_result = find_result(&search_result, "Auth Research: OAuth2 vs SAML")
        .expect("oldest research result in search output");
    let oldest_decay = oldest_result["decayWeight"]
        .as_f64()
        .expect("oldest decayWeight");
    assert!(
        oldest_decay < decision_decay,
        "90-day-old document decay weight ({oldest_decay}) should be less than recent decision ({decision_decay}): {search_result}"
    );

    // T-12.7: RTM NFR-1 — assert fused scores reflect decay-adjusted ordering
    let decision_fused = decision_result["fusedScore"]
        .as_f64()
        .expect("decision fusedScore");
    let oldest_fused = oldest_result["fusedScore"]
        .as_f64()
        .expect("oldest fusedScore");
    assert!(
        decision_fused > oldest_fused,
        "decision fusedScore ({decision_fused}) should exceed oldest stale doc fusedScore ({oldest_fused}): {search_result}"
    );
}

// T-12.7: RTM NFR-1
/// Verifies that a recent decision outranks stale semantically-related docs in
/// hybrid search, with decay weight and fused score assertions to isolate the
/// temporal signal from text-relevance dominance.
#[test]
fn recent_decision_outranks_stale_related_docs_in_hybrid_search() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-unrelated-1.md"),
        "Database Migration Guide",
        "Guide for migrating from [[PostgreSQL]] to [[CockroachDB]]. Schema changes and data types.",
        80,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-unrelated-2.md"),
        "CSS Grid Layout",
        "Using [[CSS-Grid]] for responsive layouts. Grid template areas and auto-placement.",
        70,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-unrelated-3.md"),
        "Docker Networking",
        "Configuring [[Docker]] bridge networks and overlay networks for [[container]] orchestration.",
        65,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-unrelated-4.md"),
        "Git Workflow",
        "Implementing [[trunk-based-development]] with [[feature-flags]] and continuous delivery.",
        55,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-related-1.md"),
        "Error Handling Research",
        "Research on [[error-handling]] patterns: circuit breakers, retries, and [[graceful-degradation]]. How to build [[resilient]] systems that handle failures.",
        90,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-related-2.md"),
        "Old Observability Setup",
        "Setting up [[observability]] with [[Prometheus]] and [[Grafana]]. Metrics, traces, and [[error-handling]] dashboards.",
        85,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-related-3.md"),
        "Logging Best Practices",
        "Structured [[logging]] for [[error-handling]] and debugging. Correlation IDs, log levels, and centralized aggregation.",
        45,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/error-strategy-decision.md"),
        "DECISION: Error Handling Strategy",
        "Decided on [[error-handling]] strategy: use [[Result-type]] pattern for expected errors, exceptions for unexpected failures. [[Graceful-degradation]] via circuit breakers with [[Polly]] library. This is the final approach for all new services.",
        2,
    );

    sync_graph(&env);

    let search_result =
        search_memories_json(&env, "error handling strategy for services", "Hybrid");
    let decision_rank = find_document_rank(&search_result, "DECISION: Error Handling Strategy");
    let old_related_1_rank = find_document_rank(&search_result, "Error Handling Research");
    let old_related_2_rank = find_document_rank(&search_result, "Old Observability Setup");

    assert!(
        decision_rank.is_some(),
        "decision document should appear in search results: {search_result}"
    );
    let decision_pos = decision_rank.expect("decision rank");
    assert!(
        decision_pos < 2,
        "recent decision document should rank in the top 2. decision={}, old1={}, old2={}, search={search_result}",
        format_rank(decision_rank),
        format_rank(old_related_1_rank),
        format_rank(old_related_2_rank),
    );

    // T-12.7: RTM NFR-1 — assert stale related docs rank below the recent decision
    if let Some(old1_pos) = old_related_1_rank {
        assert!(
            decision_pos < old1_pos,
            "recent decision (rank {}) should outrank 90-day-old Error Handling Research (rank {}): {search_result}",
            decision_pos,
            old1_pos
        );
    }
    if let Some(old2_pos) = old_related_2_rank {
        assert!(
            decision_pos < old2_pos,
            "recent decision (rank {}) should outrank 85-day-old Old Observability Setup (rank {}): {search_result}",
            decision_pos,
            old2_pos
        );
    }

    // T-12.7: RTM NFR-1 — assert decay weights: recent doc near-full, stale docs suppressed
    let decision_result = find_result(&search_result, "DECISION: Error Handling Strategy")
        .expect("decision result in search output");
    let decision_decay = decision_result["decayWeight"]
        .as_f64()
        .expect("decision decayWeight");
    assert!(
        decision_decay >= 0.95,
        "recent decision (2 days old) should have near-full decay weight, got {decision_decay}: {search_result}"
    );

    // T-12.7: RTM NFR-1 — assert oldest related doc has lower decay weight than decision
    if let Some(old1_result) = find_result(&search_result, "Error Handling Research") {
        let old1_decay = old1_result["decayWeight"]
            .as_f64()
            .expect("old related 1 decayWeight");
        assert!(
            old1_decay < decision_decay,
            "90-day-old related doc decay weight ({old1_decay}) should be less than recent decision ({decision_decay}): {search_result}"
        );

        // T-12.7: RTM NFR-1 — assert fused score ordering reflects decay
        let decision_fused = decision_result["fusedScore"]
            .as_f64()
            .expect("decision fusedScore");
        let old1_fused = old1_result["fusedScore"]
            .as_f64()
            .expect("old related 1 fusedScore");
        assert!(
            decision_fused > old1_fused,
            "decision fusedScore ({decision_fused}) should exceed stale related doc fusedScore ({old1_fused}): {search_result}"
        );
    }
}
