use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::vector;
use rusqlite::{
    Connection, OptionalExtension,
    types::{Type, ValueRef},
};

// T-3.1: RTM FR-6
#[allow(clippy::too_many_lines)]
pub(super) fn validate_repair_concepts_semantics(
    context: &ToolContext,
    variants: &[String],
    canonical_concept: &str,
    min_semantic_similarity: f64,
) -> Result<String> {
    if min_semantic_similarity <= 0.0 {
        return Ok(String::new());
    }

    let header = format!(
        "=== SEMANTIC VALIDATION ===\nChecking semantic similarity with threshold {:.2}...\n",
        min_semantic_similarity
    );
    let connection = db::open_connection(&context.config.database_path)?;

    // T-9.2 / T-3.1: RTM FR-8, FR-6 — load the [[vec0]] extension before any query
    // that touches vec_concepts or vec_distance_cosine(). Without this, the "no such
    // module: vec0" error would always trigger the BLOCKED path, even when the extension
    // binary is available and embeddings exist.
    if db::load_vec0_extension(&connection, &context.config.assets_path).is_err() {
        return Ok(format!(
            "{header}\n⚠️ BLOCKED: Semantic validation cannot proceed because vec0 embeddings are unavailable (vec0 extension not found).\n\nOperation blocked. Either:\n- Load an embedding model to enable vec0\n- Set minSemanticSimilarity=0 to skip semantic validation"
        ));
    }

    let canonical_embedding = match load_concept_embedding(&connection, canonical_concept) {
        Ok(Some(embedding)) => embedding,
        Ok(None) => {
            return Ok(format!(
                "{header}⚠️ Warning: Semantic validation skipped because embedding for '[[{canonical_concept}]]' was not found.\n"
            ));
        }
        // PARITY-R03: RTM NFR-1 — C# blocks with error when vec0 is unavailable;
        // match that behavior by returning a BLOCKED error instead of a warning.
        Err(error) if is_vec_embedding_unavailable(&error) => {
            return Ok(format!(
                "{header}\n⚠️ BLOCKED: Semantic validation cannot proceed because vec0 embeddings are unavailable ({error}).\n\nOperation blocked. Either:\n- Load an embedding model to enable vec0\n- Set minSemanticSimilarity=0 to skip semantic validation"
            ));
        }
        Err(error) => return Err(error),
    };

    let mut failing_variants = Vec::new();
    for variant in variants {
        let variant_embedding = match load_concept_embedding(&connection, variant) {
            Ok(Some(embedding)) => embedding,
            Ok(None) => {
                return Ok(format!(
                    "{header}⚠️ Warning: Semantic validation skipped because embedding for '[[{variant}]]' was not found.\n"
                ));
            }
            // PARITY-R03: RTM NFR-1 — C# blocks with error when vec0 is unavailable
            Err(error) if is_vec_embedding_unavailable(&error) => {
                return Ok(format!(
                    "{header}\n⚠️ BLOCKED: Semantic validation cannot proceed because vec0 embeddings are unavailable ({error}).\n\nOperation blocked. Either:\n- Load an embedding model to enable vec0\n- Set minSemanticSimilarity=0 to skip semantic validation"
                ));
            }
            Err(error) => return Err(error),
        };

        let similarity = cosine_similarity(&canonical_embedding, &variant_embedding);
        if similarity < min_semantic_similarity {
            failing_variants.push((variant.as_str(), similarity));
        }
    }

    if failing_variants.is_empty() {
        return Ok(format!(
            "{header}✓ All variants pass semantic similarity check\n\n"
        ));
    }

    let failing_lines = failing_variants
        .iter()
        .map(|(variant, similarity)| {
            format!(
                "  • {variant}: similarity {:.2} (threshold: {:.2})",
                similarity, min_semantic_similarity
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // T-14.9: RTM-042 / RTM-043 — mirror the C# oracle by returning a successful
    // BLOCKED warning payload so callers can decide whether to proceed without
    // handling this safety check as a transport-level error.
    Ok(format!(
        "{header}\n⚠️ BLOCKED: The following concepts are not semantically similar enough to '[[{canonical_concept}]]':\n{failing_lines}\n\nOperation blocked. Review the concepts and either:\n- Lower minSemanticSimilarity threshold\n- Remove the dissimilar concepts from conceptsToReplace"
    ))
}

// T-3.1: RTM FR-6
pub(crate) fn load_concept_embedding(
    connection: &Connection,
    concept_name: &str,
) -> Result<Option<Vec<f32>>> {
    let mut statement =
        connection.prepare("SELECT embedding FROM vec_concepts WHERE concept_name = ?1")?;
    statement
        .query_row([concept_name], |row| {
            decode_embedding_value(row.get_ref(0)?)
        })
        .optional()
        .map_err(Into::into)
}

fn decode_embedding_value(value: ValueRef<'_>) -> rusqlite::Result<Vec<f32>> {
    match value {
        ValueRef::Blob(bytes) => decode_blob_embedding(bytes).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Blob,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            )
        }),
        ValueRef::Text(text) => decode_text_embedding(text).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            )
        }),
        ValueRef::Null => Ok(Vec::new()),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            other.data_type(),
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "embedding column must be BLOB or TEXT",
            )),
        )),
    }
}

fn decode_blob_embedding(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    let chunks = bytes.chunks_exact(std::mem::size_of::<f32>());
    if !chunks.remainder().is_empty() {
        return Err("embedding blob length is not divisible by 4".to_string());
    }

    Ok(chunks
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn decode_text_embedding(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("embedding text is not valid UTF-8: {error}"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<f32>>(trimmed)
            .map_err(|error| format!("embedding JSON parse failed: {error}"));
    }

    trimmed
        .split([',', ' ', '\n', '\r', '\t'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<f32>()
                .map_err(|error| format!("embedding value '{part}' is not a float: {error}"))
        })
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(left, right)| (*left as f64) * (*right as f64))
        .sum();
    let magnitude_a: f64 = a
        .iter()
        .map(|value| (*value as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    let magnitude_b: f64 = b
        .iter()
        .map(|value| (*value as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot / (magnitude_a * magnitude_b)
}

// T-3.1: RTM FR-6
pub(crate) fn is_vec_embedding_unavailable(error: &MaenifoldError) -> bool {
    matches!(
        error,
        MaenifoldError::Sqlite(sqlite_error)
            if sqlite_error.to_string().contains("no such table: vec_concepts")
                || sqlite_error.to_string().contains("no such table: vec_memory_files")
                || sqlite_error.to_string().contains("no such module: vec0")
                || sqlite_error.to_string().contains("unknown module: vec0")
                || sqlite_error
                    .to_string()
                    .contains("no such function: vec_distance_cosine")
    )
}

// T-3.1: RTM FR-6
pub(crate) fn embedding_is_valid_for_similarity_search(embedding: &[f32]) -> bool {
    let expected_dims =
        vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);
    if embedding.len() != expected_dims {
        return false;
    }

    let mut sum_sq = 0.0_f64;
    for value in embedding {
        if !value.is_finite() {
            return false;
        }
        let value = f64::from(*value);
        sum_sq += value * value;
    }

    sum_sq >= 1e-12
}

// T-14.9: RTM-042 / RTM-043 — blocked semantic validation remains a successful
// response payload, but the caller still needs a reliable way to stop the repair.
pub(crate) fn semantic_validation_is_blocked(output: &str) -> bool {
    output.contains("BLOCKED:")
}
