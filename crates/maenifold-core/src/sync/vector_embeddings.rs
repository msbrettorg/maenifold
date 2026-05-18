use super::vector_diagnostics::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, vector_sync_disabled_by_config,
};
use super::vector_embeddings_helpers::{
    ensure_file_embedding_preprocessing, record_concept_failures,
    record_concept_failures_with_detail, store_file_embedding,
};
use crate::error::Result;
use crate::markdown::normalize_concept;
use crate::vector;
use rusqlite::{Connection, params, params_from_iter};
use std::collections::{BTreeSet, HashSet};
// T-8.5a: RTM FR-25.2 / FR-33.2 — full [[sync]] embedding refreshes need the deduplicated
// concept set from the rebuilt graph before generating vec_concepts rows.
pub(crate) fn load_all_concept_names(connection: &Connection) -> Result<Vec<String>> {
    let mut statement =
        connection.prepare("SELECT concept_name FROM concepts WHERE concept_name != ''")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let mut concepts = BTreeSet::new();
    for row in rows {
        concepts.insert(row?);
    }

    Ok(concepts.into_iter().collect())
}

// T-QUAL-002.6b: RTM FR-25.2
#[allow(clippy::too_many_lines)]
pub fn generate_concept_embeddings(
    connection: &Connection,
    concepts: &[String],
    vector_ready: bool,
    diagnostics: &mut VectorSyncDiagnostics,
) -> usize {
    if !vector_ready || vector_sync_disabled_by_config() {
        diagnostics.record_concept_skipped(concepts.len());
        return 0;
    }

    // T-9.3: RTM FR-7, FR-8 — derive embedding byte width from the runtime model dimensions.
    let dims = vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);
    let expected_bytes = dims * std::mem::size_of::<f32>();
    ensure_file_embedding_preprocessing(connection, expected_bytes, diagnostics);
    let unique_concepts = concepts
        .iter()
        .map(|concept| normalize_concept(concept))
        .filter(|concept| !concept.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if unique_concepts.is_empty() {
        return 0;
    }

    let mut existing_concepts = HashSet::new();
    for chunk in unique_concepts.chunks(900) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT concept_name FROM vec_concepts WHERE concept_name IN ({placeholders}) AND typeof(embedding) = 'blob' AND length(embedding) = {expected_bytes}"
        );
        let mut statement = match connection.prepare(&sql) {
            Ok(statement) => statement,
            Err(_) => {
                record_concept_failures(
                    diagnostics,
                    VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED,
                    chunk.len(),
                );
                return 0;
            }
        };
        let rows = match statement.query_map(params_from_iter(chunk.iter()), |row| {
            row.get::<_, String>(0)
        }) {
            Ok(rows) => rows,
            Err(_) => {
                record_concept_failures(
                    diagnostics,
                    VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED,
                    chunk.len(),
                );
                return 0;
            }
        };

        for row in rows {
            match row {
                Ok(concept_name) => {
                    existing_concepts.insert(concept_name);
                }
                Err(_) => {
                    record_concept_failures(
                        diagnostics,
                        VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED,
                        chunk.len(),
                    );
                    return 0;
                }
            }
        }
    }

    let missing_concepts = unique_concepts
        .into_iter()
        .filter(|concept| !existing_concepts.contains(concept))
        .collect::<Vec<_>>();
    if missing_concepts.is_empty() {
        return 0;
    }

    let missing_refs = missing_concepts
        .iter()
        .map(std::string::String::as_str)
        .collect::<Vec<_>>();
    let embeddings = match vector::generate_embeddings(&missing_refs, 32) {
        Ok(embeddings) => embeddings,
        Err(error) => {
            // T-20.1: RTM FR-7 — preserve existing [[CONCEPT_EMBEDDING_FAILED]] counters while
            // recording a small amount of actionable runtime detail for sync diagnostics.
            record_concept_failures_with_detail(
                diagnostics,
                VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
                missing_concepts.len(),
                &error.to_string(),
            );
            return 0;
        }
    };

    let mut generated = 0usize;
    let mut insert_statement = match connection
        .prepare("INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)")
    {
        Ok(statement) => statement,
        Err(_) => {
            record_concept_failures(
                diagnostics,
                VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
                missing_concepts.len(),
            );
            return 0;
        }
    };

    for (concept_name, embedding) in missing_concepts.iter().zip(embeddings.iter()) {
        let blob = vector::to_sqlite_vector_blob(embedding);
        if blob.len() != expected_bytes {
            diagnostics
                .record_concept_failure(VectorFailureReasonCodes::CONCEPT_BLOB_SIZE_MISMATCH);
            continue;
        }

        match insert_statement.execute(params![concept_name, blob]) {
            Ok(_) => generated += 1,
            Err(_) => diagnostics
                .record_concept_failure(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED),
        }
    }

    if embeddings.len() < missing_concepts.len() {
        record_concept_failures(
            diagnostics,
            VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED,
            missing_concepts.len() - embeddings.len(),
        );
    }

    generated
}

// T-8.3: RTM FR-33.3 — sync writes per-file [[embedding]] rows into vec_memory_files so
// standalone Rust [[sync]] updates semantic state without relying on C#-seeded vectors.
pub fn generate_file_embedding(
    connection: &Connection,
    file_path: &str,
    content: &str,
    vector_ready: bool,
    diagnostics: &mut VectorSyncDiagnostics,
) -> bool {
    if !vector_ready || vector_sync_disabled_by_config() {
        diagnostics.record_file_skipped();
        return false;
    }

    // T-8.4: RTM FR-25.2 — standalone Rust sync must provision vec_memory_files even when
    // sqlite-vec virtual tables were unavailable during initial database bootstrap.
    if connection
        .execute(
            "CREATE TABLE IF NOT EXISTS vec_memory_files (
                file_path TEXT PRIMARY KEY,
                embedding BLOB NOT NULL
            )",
            [],
        )
        .is_err()
    {
        diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
        return false;
    }

    // T-9.3: RTM FR-7, FR-8 — derive embedding byte width from the runtime model dimensions.
    let dims = vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);
    let expected_bytes = dims * std::mem::size_of::<f32>();
    store_file_embedding(connection, file_path, content, expected_bytes, diagnostics)
}

// T-9.2 / T-3.1: RTM FR-8, FR-6 — re-export from the shared `db::vec0` module so
// existing `crate::sync::vector_embeddings::vec0_extension_available` import paths
// continue to compile.
pub(crate) use crate::db::vec0_extension_available;
