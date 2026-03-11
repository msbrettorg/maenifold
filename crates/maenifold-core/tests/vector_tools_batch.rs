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

// T-8.4: RTM FR-33.1 — a single-item batch must match the individual embedding exactly.
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
fn fr33_1_ac2_batch_chunking_respects_max_batch_five_texts_with_max_batch_two_all_five_embedded() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let texts = ["one", "two", "three", "four", "five"];
    let embeddings = vector::generate_embeddings(&texts, 2).expect("chunked embeddings");
    assert_eq!(embeddings.len(), texts.len());
    for embedding in embeddings {
        assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS);
    }
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

// T-8.4: RTM FR-33.2 — the default max batch size of 32 still processes inputs larger than a single chunk.
#[test]
fn nfr33_2_ac2_default_max_batch_is_32_larger_input_set_still_produces_all_embeddings() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let texts = (0..40)
        .map(|index| format!("concept sample {index}"))
        .collect::<Vec<_>>();
    let refs = texts.iter().map(String::as_str).collect::<Vec<_>>();
    let embeddings = vector::generate_embeddings(&refs, 0).expect("default-batch embeddings");
    assert_eq!(embeddings.len(), texts.len());
    for embedding in embeddings {
        assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS);
    }
}

// T-8.4: RTM FR-25.2 / FR-33.2 — vec_concepts existence checks are scoped to requested concepts so unrelated rows do not skip new embeddings.
#[test]
fn fr33_2_ac4_scoped_vec_concepts_lookup_unrelated_existing_concepts_do_not_cause_new_concepts_to_be_skipped()
 {
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
