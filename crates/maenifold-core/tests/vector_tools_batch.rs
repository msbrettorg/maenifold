#[path = "vector_tools/support.rs"]
mod vector_tools_support;

use maenifold_core::db;
use maenifold_core::search;
use maenifold_core::vector;
use rusqlite::params;
use vector_tools_support::{
    TEST_MUTEX, assert_embeddings_match, ensure_regular_vec_concepts_table, isolated_env,
};

// T-12.6: RTM NFR-1 — batch/vector sync regressions remain isolated after the file split.

// T-8.4: RTM FR-33.1 — a single-item batch with max_batch=1 must match the individual
// embedding exactly.
//
// NOTE: `src/vector/tests.rs::vector_batch_matches_individual` tests the same parity claim
// internally using `DEFAULT_MAX_BATCH` and a 1e-6 tolerance. This integration test adds:
//   1. `TEST_MUTEX` serialization (prevents inter-test ONNX model contention),
//   2. `vector::cleanup()` pre-condition (resets model state), and
//   3. `f32::EPSILON` tolerance via `assert_embeddings_match` (stricter than 1e-6).
// If the internal test is ever strengthened with mutex serialization, consider removing this test.
#[test]
fn fr33_1_ac1_batch_produces_identical_output() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let text = "batch parity example";
    let individual = vector::generate_embedding(text).expect("individual embedding");
    let batched = vector::generate_embeddings(&[text], 1).expect("single-item batch");
    assert_eq!(batched.len(), 1);
    assert_embeddings_match(&individual, &batched[0]);
}

// T-8.4: RTM FR-33.1 — chunked batch execution with max_batch=2 still embeds every requested text.
#[test]
fn fr33_1_ac2_batch_chunking_respects_max_batch() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let texts = ["one", "two", "three", "four", "five"];
    let embeddings = vector::generate_embeddings(&texts, 2).expect("chunked embeddings");
    assert_eq!(embeddings.len(), texts.len(), "all 5 texts must produce embeddings");
    for (index, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS);
        // T-8.4: FR-33.1 — guard against silent zero-vector regressions in the chunked path.
        assert!(
            embedding.iter().any(|v| v.abs() > f32::EPSILON),
            "embedding[{index}] for {:?} is a zero-vector",
            texts[index],
        );
    }
    // T-8.4: FR-33.1 — distinct inputs must produce distinct embeddings.
    assert_ne!(
        embeddings[0], embeddings[1],
        "distinct inputs 'one' and 'two' must produce different embeddings",
    );
}

// T-8.4: RTM FR-33.1 — empty/blank inputs preserve the zero-vector behavior in both individual and batched APIs.
#[test]
fn fr33_1_ac3_batch_empty_null_inputs_produce_zero_vectors_matching_individual_behavior() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let individual_empty = vector::generate_embedding("").expect("empty individual");
    let individual_blank = vector::generate_embedding("   ").expect("blank individual");
    let batched = vector::generate_embeddings(&["", "   "], 2).expect("empty batch");
    let expected_zero = vec![0.0; vector::EMBEDDING_DIMENSIONS];
    assert_eq!(batched.len(), 2);
    assert_eq!(individual_empty, expected_zero);
    assert_eq!(individual_blank, expected_zero);
    assert_eq!(batched[0], expected_zero);
    assert_eq!(batched[1], expected_zero);
}

// T-8.4: RTM FR-33.2 — max_batch=0 triggers the DEFAULT_MAX_BATCH fallback (32, pub(crate) so
// not directly assertable from integration tests) and all embeddings are returned even when
// the input count (33) exceeds a single default batch.
#[test]
fn fr33_2_ac2_default_fallback_processes_all_embeddings() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    // 33 = DEFAULT_MAX_BATCH (32) + 1, forcing a two-chunk split at the boundary.
    let texts = (0..33)
        .map(|index| format!("concept sample {index}"))
        .collect::<Vec<_>>();
    let refs = texts.iter().map(String::as_str).collect::<Vec<_>>();
    let embeddings = vector::generate_embeddings(&refs, 0).expect("default-batch embeddings");
    assert_eq!(embeddings.len(), texts.len(), "all 33 texts must produce embeddings");
    for (index, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS);
        // T-8.4: FR-33.2 — guard against silent zero-vector regressions in the default path.
        assert!(
            embedding.iter().any(|v| v.abs() > f32::EPSILON),
            "embedding[{index}] is a zero-vector",
        );
    }
}

// T-8.4: RTM FR-25.2 / FR-33.2 — vec_concepts existence checks are scoped to requested concepts so unrelated rows do not skip new embeddings.
#[test]
fn fr33_2_ac4_scoped_vec_lookup_skips_only_existing_concepts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    ensure_regular_vec_concepts_table(&env);
    let connection = db::open_connection(&env.cfg.database_path).expect("open db");
    let existing = vector::generate_embedding("existing concept").expect("existing embedding");
    connection
        .execute(
            "INSERT INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
            params!["existing-concept", vector::to_sqlite_vector_blob(&existing)],
        )
        .expect("insert unrelated concept");

    let mut diagnostics = search::VectorSyncDiagnostics::default();
    let generated = search::generate_concept_embeddings(
        &connection,
        &[String::from("new-concept")],
        true,
        &mut diagnostics,
    );
    assert_eq!(generated, 1, "new concept should still be embedded");

    let blob = connection
        .query_row(
            "SELECT embedding FROM vec_concepts WHERE concept_name = ?1",
            params!["new-concept"],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("embedded concept row");
    assert_eq!(
        blob.len(),
        vector::EMBEDDING_DIMENSIONS * std::mem::size_of::<f32>()
    );
}
