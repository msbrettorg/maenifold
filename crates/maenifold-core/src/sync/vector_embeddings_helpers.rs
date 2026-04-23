//! T-01.17: helper functions extracted from [`super::vector_embeddings`] to keep
//! the parent file under the per-file LOC budget. No behavioral changes.

use super::vector_diagnostics::{VectorFailureReasonCodes, VectorSyncDiagnostics};
use crate::db;
use crate::markdown::strip_wikilink_brackets;
use crate::vector;
use rusqlite::{Connection, params};

const MODEL_METADATA_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS model_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;
const FILE_EMBEDDING_PREPROCESSING_VERSION_KEY: &str = "file_embedding_preprocessing_version";
const FILE_EMBEDDING_PREPROCESSING_VERSION: &str = "wikilink-strip-v1";

pub(super) fn record_concept_failures(
    diagnostics: &mut VectorSyncDiagnostics,
    reason: &str,
    count: usize,
) {
    let effective = count.max(1);
    for _ in 0..effective {
        diagnostics.record_concept_failure(reason);
    }
}

pub(super) fn record_concept_failures_with_detail(
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

pub(super) fn ensure_file_embedding_preprocessing(
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

pub(super) fn store_file_embedding(
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
