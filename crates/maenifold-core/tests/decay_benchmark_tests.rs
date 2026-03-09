use chrono::{Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory::{self, compute_decay_weight};
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "decay-benchmark-tests";

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn memory_uri_from_relative(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn create_test_file_with_age(
    cfg: &Config,
    relative_path: &str,
    title: &str,
    content: &str,
    days_ago: i64,
) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    let parent = file_path.parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create test dirs");

    let created_iso = rfc3339_days_ago(days_ago);
    let escaped_title = title.replace('"', "\\\"");
    let full_content = format!(
        "---\n\
         title: \"{escaped_title}\"\n\
         type: \"memory\"\n\
         status: \"saved\"\n\
         created: {created_iso}\n\
         modified: {created_iso}\n\
         ---\n\n\
         # {title}\n\n\
         {content}\n"
    );
    fs::write(&file_path, full_content).expect("write memory file");

    memory_uri_from_relative(relative_path)
}

fn sync_graph(env: &TestEnv) {
    let output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
}

fn search_memories_json(env: &TestEnv, query: &str, mode: &str) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": query,
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str(&raw).expect("search memories json")
}

fn read_memory_markdown(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx_markdown(), &json!({ "identifier": identifier })).expect("read")
}

fn decay_weight_for_uri(cfg: &Config, uri: &str) -> f64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    let (last_accessed, status) = connection
        .query_row(
            "SELECT last_accessed, status FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .expect("query decay metadata");

    compute_decay_weight(uri, last_accessed.as_deref(), cfg, status.as_deref())
}

fn find_document_rank(search_result: &Value, title_fragment: &str) -> Option<usize> {
    search_result["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .position(|entry| {
            entry["title"]
                .as_str()
                .map(|title| title.contains(title_fragment))
                .unwrap_or(false)
        })
}

fn format_rank(rank: Option<usize>) -> String {
    rank.map(|value| format!("#{}", value + 1))
        .unwrap_or_else(|| "NOT FOUND".to_string())
}

fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

// T-0.1: RTM NFR-3 - Ports DecayBenchmarkTests.cs coverage to guard benchmark-style decay regressions.
// T-GRAPH-DECAY-BENCH-001: RTM FR-4.4
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

// T-0.1: RTM NFR-3 - Ports DecayBenchmarkTests.cs coverage to guard benchmark-style decay regressions.
// T-GRAPH-DECAY-BENCH-001: RTM FR-4.4
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

// T-0.1: RTM NFR-3 - Ports DecayBenchmarkTests.cs coverage to guard benchmark-style decay regressions.
// T-GRAPH-DECAY-BENCH-001: RTM FR-4.4
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

// T-0.1: RTM NFR-3 - Ports DecayBenchmarkTests.cs coverage to guard benchmark-style decay regressions.
// T-GRAPH-DECAY-BENCH-001: RTM FR-4.4
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

// T-0.1: RTM NFR-3 - Ports DecayBenchmarkTests.cs coverage to guard benchmark-style decay regressions.
// T-GRAPH-DECAY-BENCH-001: RTM FR-4.4
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
