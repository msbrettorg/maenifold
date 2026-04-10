use super::vector_diagnostics::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, vector_sync_disabled_by_config,
};
use crate::db;
use crate::error::Result;
use crate::markdown::{normalize_concept, strip_wikilink_brackets};
use crate::vector;
use rusqlite::{Connection, params, params_from_iter};
use std::collections::{BTreeSet, HashSet};

const MODEL_METADATA_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS model_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;
const FILE_EMBEDDING_PREPROCESSING_VERSION_KEY: &str = "file_embedding_preprocessing_version";
const FILE_EMBEDDING_PREPROCESSING_VERSION: &str = "wikilink-strip-v1";

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
            // sampling a small amount of actionable runtime detail for sync diagnostics.
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

fn record_concept_failures(diagnostics: &mut VectorSyncDiagnostics, reason: &str, count: usize) {
    let effective = count.max(1);
    for _ in 0..effective {
        diagnostics.record_concept_failure(reason);
    }
}

fn record_concept_failures_with_detail(
    diagnostics: &mut VectorSyncDiagnostics,
    reason: &str,
    count: usize,
    detail: &str,
) {
    let effective = count.max(1);
    diagnostics.record_concept_failure_with_detail(reason, detail);
    for _ in 1..effective {
        diagnostics.record_concept_failure(reason);
    }
}

fn ensure_file_embedding_preprocessing(
    connection: &Connection,
    expected_bytes: usize,
    diagnostics: &mut VectorSyncDiagnostics,
) {
    if connection.execute_batch(MODEL_METADATA_TABLE_SQL).is_err() {
        diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
        return;
    }

    if db::get_model_metadata(connection, FILE_EMBEDDING_PREPROCESSING_VERSION_KEY).as_deref()
        == Some(FILE_EMBEDDING_PREPROCESSING_VERSION)
    {
        return;
    }

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
        return;
    }

    let mut rows = match connection.prepare("SELECT file_path, content FROM file_content") {
        Ok(statement) => statement,
        Err(_) => {
            diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
            return;
        }
    };
    let file_rows = match rows.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(rows) => rows,
        Err(_) => {
            diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
            return;
        }
    };

    let mut files = Vec::new();
    for row in file_rows {
        match row {
            Ok(file) => files.push(file),
            Err(_) => {
                diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
                return;
            }
        }
    }

    if connection
        .execute("DELETE FROM vec_memory_files", [])
        .is_err()
    {
        diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
        return;
    }

    for (file_path, content) in files {
        let _ = store_file_embedding(
            connection,
            &file_path,
            &content,
            expected_bytes,
            diagnostics,
        );
    }

    if connection
        .execute(
            "INSERT INTO model_metadata (key, value)
             VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![
                FILE_EMBEDDING_PREPROCESSING_VERSION_KEY,
                FILE_EMBEDDING_PREPROCESSING_VERSION
            ],
        )
        .is_err()
    {
        diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
    }
}

fn store_file_embedding(
    connection: &Connection,
    file_path: &str,
    content: &str,
    expected_bytes: usize,
    diagnostics: &mut VectorSyncDiagnostics,
) -> bool {
    let stripped = strip_wikilink_brackets(content);
    let text_to_embed = truncate_for_file_embedding(&stripped, 1000);
    let embedding = match vector::generate_embedding(text_to_embed) {
        Ok(embedding) => embedding,
        Err(_) => {
            diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
            return false;
        }
    };
    let blob = vector::to_sqlite_vector_blob(&embedding);
    if blob.len() != expected_bytes {
        diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_BLOB_SIZE_MISMATCH);
        return false;
    }

    match connection.execute(
        "INSERT OR REPLACE INTO vec_memory_files (file_path, embedding) VALUES (?1, ?2)",
        params![file_path, blob],
    ) {
        Ok(_) => true,
        Err(_) => {
            diagnostics.record_file_failure(VectorFailureReasonCodes::FILE_EMBEDDING_FAILED);
            false
        }
    }
}

fn truncate_for_file_embedding(content: &str, max_chars: usize) -> &str {
    if content.chars().count() <= max_chars {
        return content;
    }

    match content.char_indices().nth(max_chars) {
        Some((index, _)) => &content[..index],
        None => content,
    }
}
