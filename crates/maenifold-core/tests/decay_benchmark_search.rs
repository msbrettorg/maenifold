// T-12.7: RTM NFR-1
#[path = "decay_benchmark/support.rs"]
mod decay_benchmark_support;

use decay_benchmark_support::{
    TEST_FOLDER, TEST_MUTEX, create_test_file_with_age, find_document_rank, format_rank,
    isolated_env, search_memories_json, sync_graph,
};

// T-12.7: RTM NFR-1
#[test]
fn benchmark_context_rot_decay_prevents_old_noise_drowning_recent_signal() {
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
}

// T-12.7: RTM NFR-1
#[test]
fn benchmark_precision_at_k_relevant_recent_doc_in_top_3() {
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
    assert!(
        decision_rank.expect("decision rank") < 3,
        "recent decision document should rank in the top 3. decision={}, old1={}, old2={}, search={search_result}",
        format_rank(decision_rank),
        format_rank(old_related_1_rank),
        format_rank(old_related_2_rank),
    );
}
