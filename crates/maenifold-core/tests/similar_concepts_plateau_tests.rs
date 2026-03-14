use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase, MaenifoldError};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn markdown_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
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

/// Helper for tests that expect `Ok` — panics on `Err` with context.
fn find_similar_concepts(env: &TestEnv, concept_name: &str, max_results: u64) -> String {
    search::find_similar_concepts(
        &env.markdown_context(),
        &json!({
            "conceptName": concept_name,
            "maxResults": max_results,
        }),
    )
    .expect("find similar concepts")
}

/// Helper for tests that need to inspect the `Result` directly (e.g. error paths).
fn try_find_similar_concepts(
    env: &TestEnv,
    concept_name: &str,
    max_results: u64,
) -> Result<String, MaenifoldError> {
    search::find_similar_concepts(
        &env.markdown_context(),
        &json!({
            "conceptName": concept_name,
            "maxResults": max_results,
        }),
    )
}

/// Seed the concept_graph table with a co-occurrence edge for graph-based search.
fn seed_concept_graph(connection: &Connection, concept_a: &str, concept_b: &str, count: i64) {
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
            params![concept_a],
        )
        .expect("insert concept_a");
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name) VALUES (?1)",
            params![concept_b],
        )
        .expect("insert concept_b");
    connection
        .execute(
            "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count) VALUES (?1, ?2, ?3)",
            params![concept_a, concept_b, count],
        )
        .expect("insert concept_graph edge");
}

fn parse_similarity_values(output: &str) -> Vec<f64> {
    output
        .lines()
        .filter_map(|line| {
            let (_, suffix) = line.split_once("similarity:")?;
            let number = suffix.split(')').next()?.trim();
            number.parse::<f64>().ok()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// KEEP: Tests 1–3 (validation guards — input normalization & bracket checks)
// ---------------------------------------------------------------------------

#[test]
fn find_similar_concepts_when_query_is_punctuation_only_does_not_emit_confident_results() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = find_similar_concepts(&env, "----", 10);

    assert!(
        result.contains("DIAGNOSTIC:"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("too low-information"),
        "unexpected output: {result}"
    );
    assert!(
        !result.contains("Similar concepts to"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("Next: Consider using BuildContext, SearchMemories"),
        "unexpected output: {result}"
    );
}

#[test]
fn find_similar_concepts_when_query_contains_unicode_bracket_confusables_rejects_helpfully() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = find_similar_concepts(&env, "［［tool］］", 10);

    assert!(
        result.contains("DIAGNOSTIC:"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("contains bracket characters"),
        "unexpected output: {result}"
    );
    assert!(
        !result.contains("Similar concepts to"),
        "unexpected output: {result}"
    );
}

#[test]
fn find_similar_concepts_when_query_contains_bracket_like_delimiters_rejects_helpfully() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = find_similar_concepts(&env, "⟦tool⟧", 10);

    assert!(
        result.contains("DIAGNOSTIC:"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("bracket-like delimiter"),
        "unexpected output: {result}"
    );
    assert!(
        !result.contains("Similar concepts to"),
        "unexpected output: {result}"
    );
}

// ---------------------------------------------------------------------------
// REWRITE: Test 4 — concept_name length validation
// The original panicked because the test helper used `.expect()` on an Err
// return. The production code returns Err(MaenifoldError::Message(...)) for
// over-limit names, not Ok("ERROR: ...").
// ---------------------------------------------------------------------------

#[test]
fn find_similar_concepts_when_concept_name_too_long_returns_validation_error() {
    // T-QUAL-FSC2: RTM FR-7.4 — validates 257-char conceptName is rejected
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let too_long = "a".repeat(257);
    let result = try_find_similar_concepts(&env, &too_long, 10);

    assert!(result.is_err(), "257-char conceptName must return Err");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("conceptName must be 256 characters or fewer"),
        "unexpected error message: {err}"
    );
}

#[test]
fn find_similar_concepts_when_concept_name_at_limit_succeeds() {
    // T-QUAL-FSC2: RTM FR-7.4 — boundary: exactly 256 chars passes validation
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let at_limit = "a".repeat(256);
    let result = try_find_similar_concepts(&env, &at_limit, 10);

    assert!(
        result.is_ok(),
        "256-char conceptName must pass validation, got: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// REWRITE: Test 5 — maxResults range validation
// Same root cause as Test 4: `.expect()` panicked on Err returns.
// Split into independent tests so each failure is independently observable.
// ---------------------------------------------------------------------------

#[test]
fn find_similar_concepts_when_max_results_is_zero_returns_validation_error() {
    // T-QUAL-FSC2: RTM FR-7.4 — validates maxResults=0 is rejected
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = try_find_similar_concepts(&env, "test", 0);

    assert!(result.is_err(), "maxResults=0 must return Err");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("maxResults must be between 1 and 50"),
        "unexpected error message: {err}"
    );
}

#[test]
fn find_similar_concepts_when_max_results_above_fifty_returns_validation_error() {
    // T-QUAL-FSC2: RTM FR-7.4 — validates maxResults=51 is rejected
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = try_find_similar_concepts(&env, "test", 51);

    assert!(result.is_err(), "maxResults=51 must return Err");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("maxResults must be between 1 and 50"),
        "unexpected error message: {err}"
    );
}

#[test]
fn find_similar_concepts_when_max_results_at_boundaries_succeeds() {
    // T-QUAL-FSC2: RTM FR-7.4 — boundary: maxResults=1 and maxResults=50 pass validation
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result_one = try_find_similar_concepts(&env, "test", 1);
    let result_fifty = try_find_similar_concepts(&env, "test", 50);

    assert!(
        result_one.is_ok(),
        "maxResults=1 must pass validation, got: {:?}",
        result_one.err()
    );
    assert!(
        result_fifty.is_ok(),
        "maxResults=50 must pass validation, got: {:?}",
        result_fifty.err()
    );
}

// ---------------------------------------------------------------------------
// REWRITE: Test 6 — similarity values bounded to [0.0, 1.0]
// The original seeded vec_concepts directly but vec0 extension is unavailable
// in the test environment, causing a silent fallback to graph-based search
// which found nothing and panicked. Rewritten to use graph-based search via
// concept_graph seeding, which works without vec0.
// ---------------------------------------------------------------------------

#[test]
fn find_similar_concepts_similarity_values_are_bounded_to_one() {
    // T-QUAL-FSC2: RTM FR-7.4 — similarity scores in output must be in [0.0, 1.0]
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let connection = Connection::open(&env.cfg.database_path).expect("open db");
    seed_concept_graph(&connection, "test", "alpha", 10);
    seed_concept_graph(&connection, "test", "beta", 7);
    seed_concept_graph(&connection, "test", "gamma", 3);

    let result = find_similar_concepts(&env, "test", 10);
    assert!(
        result.contains("Similar concepts to"),
        "expected ranked results, got: {result}"
    );

    let similarity_values = parse_similarity_values(&result);
    assert!(
        !similarity_values.is_empty(),
        "expected parseable similarity values in output: {result}"
    );

    for similarity in &similarity_values {
        assert!(
            *similarity >= 0.0 && *similarity <= 1.0,
            "similarity must be in [0.0, 1.0], got {similarity:.6}. Output:\n{result}"
        );
    }
}
