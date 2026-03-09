use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
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

fn deterministic_embedding_blob(concept: &str) -> Vec<u8> {
    let mut embedding = [0.0_f32; 384];
    for (index, byte) in concept.bytes().enumerate() {
        embedding[index % embedding.len()] += f32::from(byte) / 255.0;
    }

    embedding
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>()
}

fn seed_vec_concept(connection: &Connection, concept: &str) {
    let blob = deterministic_embedding_blob(concept);
    connection
        .execute(
            "DELETE FROM vec_concepts WHERE concept_name = ?1",
            params![concept],
        )
        .expect("delete vec concept");
    connection
        .execute(
            "INSERT INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept, blob],
        )
        .expect("insert vec concept");
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

#[test]
fn find_similar_concepts_when_concept_name_too_long_returns_validation_error() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let too_long = "a".repeat(257);
    let result = find_similar_concepts(&env, &too_long, 10);

    assert_eq!(result, "ERROR: conceptName must be 256 characters or fewer");
}

#[test]
fn find_similar_concepts_when_max_results_out_of_range_returns_validation_error() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result_low = find_similar_concepts(&env, "test", 0);
    let result_high = find_similar_concepts(&env, "test", 51);

    assert_eq!(result_low, "ERROR: maxResults must be between 1 and 50");
    assert_eq!(result_high, "ERROR: maxResults must be between 1 and 50");
}

#[test]
#[ignore = "ONNX/vector-extension dependent"]
fn find_similar_concepts_when_it_returns_results_never_emits_similarity_greater_than_one() {
    // T-QUAL-FSC2: RTM FR-7.4
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let connection = Connection::open(&env.cfg.database_path).expect("open db");
    seed_vec_concept(&connection, "alpha");
    seed_vec_concept(&connection, "beta");
    seed_vec_concept(&connection, "gamma");

    let result = find_similar_concepts(&env, "test", 10);
    assert!(
        result.contains("Similar concepts to"),
        "unexpected: {result}"
    );

    let similarity_values = parse_similarity_values(&result);
    assert!(
        !similarity_values.is_empty(),
        "expected similarity values in output: {result}"
    );

    for similarity in similarity_values {
        assert!(
            similarity <= 1.0,
            "similarity must be <= 1.000, got {similarity:.6}. Output:\n{result}"
        );
    }
}
