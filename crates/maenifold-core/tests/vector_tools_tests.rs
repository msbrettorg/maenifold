use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::fs;
use std::thread;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "vector-tools-tests";
const ONNX_GAP_REASON: &str =
    "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7";

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
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

fn write_memory_fixture(env: &TestEnv, relative_path: &str, content: &str) {
    let path = env.cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture dirs");
    fs::write(path, content).expect("fixture write");
}

fn ensure_regular_vec_concepts_table(env: &TestEnv) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute_batch(
            r#"
            DROP TABLE IF EXISTS vec_concepts;
            CREATE TABLE vec_concepts (
                concept_name TEXT PRIMARY KEY,
                embedding BLOB
            );
            "#,
        )
        .expect("create vec_concepts table");
}

fn insert_blob_embedding(env: &TestEnv, concept_name: &str, embedding: &[f32]) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, encode_blob_embedding(embedding)],
        )
        .expect("insert blob embedding");
}

fn insert_json_embedding(env: &TestEnv, concept_name: &str, embedding: &[f32]) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    let payload = serde_json::to_string(embedding).expect("serialize embedding");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, payload],
        )
        .expect("insert json embedding");
}

fn insert_text_embedding(env: &TestEnv, concept_name: &str, embedding: &str) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params![concept_name, embedding],
        )
        .expect("insert text embedding");
}

fn insert_null_embedding(env: &TestEnv, concept_name: &str) {
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, NULL)",
            params![concept_name],
        )
        .expect("insert null embedding");
}

fn invoke_repair_concepts(env: &TestEnv, payload: serde_json::Value) -> maenifold_core::Result<String> {
    search::repair_concepts(&env.ctx_markdown(), &payload)
}

fn deterministic_embedding(seed: f32) -> Vec<f32> {
    (0..384)
        .map(|index| {
            let step = ((index % 11) as f32 - 5.0) / 16.0;
            seed + step
        })
        .collect()
}

fn exact_precision_embedding() -> Vec<f32> {
    (0..384)
        .map(|index| match index % 4 {
            0 => 0.0,
            1 => 0.5,
            2 => 1.0,
            _ => -0.5,
        })
        .collect()
}

fn encode_blob_embedding(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn ignored_onnx_gap(test_name: &str) -> ! {
    unimplemented!("{test_name}: {ONNX_GAP_REASON}")
}

// T-0.1: RTM NFR-3 — Ports the C# VectorToolsTests file into the Rust oracle.
// Rust currently only supports search.rs vector decoding + semantic validation, so
// direct ONNX generation/batching tests are marked ignored until T-7.* lands.

// T-0.1: RTM NFR-3 — Adapted from C# GenerateEmbeddingProducesConsistent384DimensionalVectors
// to assert search.rs semantic validation accepts stable 384-dim vec_concepts rows.
#[test]
fn generate_embedding_produces_consistent_384_dimensional_vectors() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/consistent.md"),
        "# Test\nUsing [[tools]] in a repair scenario.\n",
    );

    let embedding = deterministic_embedding(0.25);
    insert_blob_embedding(&env, "tool", &embedding);
    insert_blob_embedding(&env, "tools", &embedding);

    let payload = json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "dryRun": true,
        "minSemanticSimilarity": 0.99
    });
    let first = invoke_repair_concepts(&env, payload.clone()).expect("first validation run");
    let second = invoke_repair_concepts(&env, payload).expect("second validation run");

    assert_eq!(first, second, "semantic validation output should be deterministic");
    assert!(
        first.contains("SEMANTIC VALIDATION")
            && first.contains("All variants pass semantic similarity check"),
        "expected successful semantic validation output: {first}"
    );
    assert!(
        first.contains("Would modify:")
            && first.contains("Total replacements: 1"),
        "expected dry-run replacement summary: {first}"
    );
}

// T-3.2: RTM FR-6 / RTM-043 — Adapted from C# FallbackEmbeddingGeneratesValidVectorsWhenONNXUnavailable
// to assert search.rs gracefully degrades when semantic validation cannot load vec embeddings.
#[test]
fn fallback_embedding_generates_valid_vectors_when_onnx_unavailable() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/fallback.md"),
        "# Test\nUsing [[tools]] without seeded vec rows.\n",
    );

    let output = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.99
        }),
    )
    .expect("repair concepts should gracefully degrade without vec rows");

    let lower = output.to_ascii_lowercase();
    assert!(
        output.contains("SEMANTIC VALIDATION"),
        "expected validation header: {output}"
    );
    assert!(
        lower.contains("skipped") || lower.contains("unavailable") || lower.contains("not found"),
        "expected graceful degradation message: {output}"
    );
    assert!(
        output.contains("=== SUMMARY ==="),
        "expected repair summary after graceful degradation: {output}"
    );
}

// T-3.2: RTM FR-6 / RTM-043 — Adapted from C# Base64EncodingPreservesEmbeddingPrecision.
// Rust search.rs consumes vec_concepts BLOB/TEXT rows instead of C# base64, so this port
// verifies precision is preserved across the supported SQLite encodings.
#[test]
fn base64_encoding_preserves_embedding_precision() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/precision.md"),
        "# Test\nUsing [[tools]] for cross-encoding precision checks.\n",
    );

    let embedding = exact_precision_embedding();
    insert_json_embedding(&env, "tool", &embedding);
    insert_blob_embedding(&env, "tools", &embedding);

    let output = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.999999
        }),
    )
    .expect("cross-encoding semantic validation");

    assert!(
        output.contains("All variants pass semantic similarity check"),
        "expected exact precision to survive JSON/BLOB decoding: {output}"
    );
}

// T-3.2: RTM FR-6 / RTM-042 — Adapted from C# EmptyAndNullInputsProduceSafeEmbeddings.
// Rust search.rs treats NULL/empty embeddings as empty vectors; this must fail safely with
// a semantic-validation error instead of crashing during cosine computation.
#[test]
fn empty_and_null_inputs_produce_safe_embeddings() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/empty-null.md"),
        "# Test\nUsing [[tools]] with empty vec rows.\n",
    );

    insert_null_embedding(&env, "tool");
    insert_text_embedding(&env, "tools", "");

    let error = invoke_repair_concepts(
        &env,
        json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "dryRun": true,
            "minSemanticSimilarity": 0.10
        }),
    )
    .expect_err("empty vectors should block safely instead of panicking");

    let message = error.to_string();
    assert!(
        message.contains("BLOCKED") || message.to_ascii_lowercase().contains("similarity"),
        "expected safe semantic-validation failure, got: {message}"
    );
}

// T-0.1: RTM NFR-3 — Adapted from C# ThreadSafeInitializationWorks to assert concurrent
// semantic-validation reads over search.rs remain stable for the same vec_concepts rows.
#[test]
fn thread_safe_initialization_works() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    write_memory_fixture(
        &env,
        &format!("{TEST_FOLDER}/thread-safe.md"),
        "# Test\nUsing [[tools]] in concurrent dry-run repairs.\n",
    );

    let embedding = deterministic_embedding(0.75);
    insert_blob_embedding(&env, "tool", &embedding);
    insert_blob_embedding(&env, "tools", &embedding);

    let handles = (0..10)
        .map(|_| {
            let cfg = env.cfg.clone();
            thread::spawn(move || {
                let context = ToolContext::new(cfg, OutputFormat::Markdown);
                search::repair_concepts(
                    &context,
                    &json!({
                        "conceptsToReplace": "tools",
                        "canonicalConcept": "tool",
                        "dryRun": true,
                        "minSemanticSimilarity": 0.99
                    }),
                )
                .expect("threaded repair concepts")
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let output = handle.join().expect("thread join");
        assert!(
            output.contains("All variants pass semantic similarity check")
                && output.contains("Total replacements: 1"),
            "each thread should observe the same successful validation: {output}"
        );
    }
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust wires native model loading.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn vector_tools_load_model_handles_case_sensitive_file_systems() {
    ignored_onnx_gap("VectorToolsLoadModelHandlesCaseSensitiveFileSystems");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust wires native token truncation.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn very_long_inputs_are_truncated_to_max_tokens() {
    ignored_onnx_gap("VeryLongInputsAreTruncatedToMaxTokens");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes native model path loading.
#[test]
#[cfg(unix)]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn path_resolution_works_on_unix_systems() {
    ignored_onnx_gap("PathResolutionWorksOnUnixSystems");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes embedding diagnostics.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn diagnostic_logging_provides_useful_information() {
    ignored_onnx_gap("DiagnosticLoggingProvidesUsefulInformation");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes ONNX inference reuse.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn onnx_session_handles_multiple_inferences() {
    ignored_onnx_gap("OnnxSessionHandlesMultipleInferences");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes model cleanup/reload hooks.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn cleanup_properly_disposes_resources() {
    ignored_onnx_gap("CleanupProperlyDisposesResources");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust wires tokenizer vocabulary loading.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn vocabulary_loading_handles_missing_file() {
    ignored_onnx_gap("VocabularyLoadingHandlesMissingFile");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust can compare ONNX vs fallback quality.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn embedding_quality_differs_between_onnx_and_fallback() {
    ignored_onnx_gap("EmbeddingQualityDiffersBetweenOnnxAndFallback");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes batch embedding generation.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn fr33_1_ac1_batch_produces_identical_output() {
    ignored_onnx_gap("FR33_1_AC1_BatchProducesIdenticalOutput");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes batch chunking controls.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn fr33_1_ac2_batch_chunking_respects_max_batch_five_texts_with_max_batch_two_all_five_embedded() {
    ignored_onnx_gap("FR33_1_AC2_BatchChunkingRespectsMaxBatch_FiveTextsWithMaxBatchTwo_AllFiveEmbedded");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes batch empty/null generation.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn fr33_1_ac3_batch_empty_null_inputs_produce_zero_vectors_matching_individual_behavior() {
    ignored_onnx_gap("FR33_1_AC3_BatchEmptyNullInputsProduceZeroVectors_MatchingIndividualBehavior");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes bit-for-bit batch inference parity.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn nfr33_1_ac1_batch_and_individual_embeddings_are_bit_for_bit_identical_single_text() {
    ignored_onnx_gap("NFR33_1_AC1_BatchAndIndividualEmbeddingsAreBitForBitIdentical_SingleText");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust exposes the default maxBatch=32 contract.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn nfr33_2_ac2_default_max_batch_is_32_larger_input_set_still_produces_all_embeddings() {
    ignored_onnx_gap("NFR33_2_AC2_DefaultMaxBatchIs32_LargerInputSetStillProducesAllEmbeddings");
}

// T-7.1: RTM NFR-3 — C# parity port placeholder until Rust sync generates/scopes concept vectors.
#[test]
#[ignore = "ONNX embedding generation is not implemented in Rust yet; tracked by T-7.1..T-7.7"]
fn fr33_2_ac4_scoped_vec_concepts_lookup_unrelated_existing_concepts_do_not_cause_new_concepts_to_be_skipped()
{
    ignored_onnx_gap("FR33_2_AC4_ScopedVecConceptsLookup_UnrelatedExistingConceptsDoNotCauseNewConceptsToBeSkipped");
}
