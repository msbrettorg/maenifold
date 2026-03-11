use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::repair::{
    embedding_is_valid_for_similarity_search, is_vec_embedding_unavailable, load_concept_embedding,
};
use crate::vector;
use rusqlite::Connection;
use rusqlite::params;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct SimilarConceptMatch {
    pub(crate) concept: String,
    pub(crate) distance: f64,
    pub(crate) similarity: f64,
}

pub(crate) fn query_vec_similar_concepts(
    connection: &Connection,
    assets_path: &Path,
    concept_name: &str,
    max_results: usize,
) -> Result<Option<Vec<SimilarConceptMatch>>> {
    // T-9.2 / T-3.1: RTM FR-8, FR-6 — use shared load_vec0_extension helper
    // instead of inline unsafe block, eliminating duplication.
    if db::load_vec0_extension(connection, assets_path).is_err() {
        return Ok(None);
    }

    // T-8.3: RTM FR-33.2 — FindSimilarConcepts can probe [[embedding]] similarity even when
    // the query concept is not yet persisted in vec_concepts.
    let query_blob = match load_concept_embedding(connection, concept_name) {
        Ok(Some(embedding))
            if !embedding.is_empty() && embedding_is_valid_for_similarity_search(&embedding) =>
        {
            vector::to_sqlite_vector_blob(&embedding)
        }
        Ok(Some(_)) => {
            return Err(MaenifoldError::InvalidArgument(
                "unable to generate a valid embedding for conceptName".to_string(),
            ));
        }
        // T-8.5a: RTM FR-7.4 / FR-33.2 — on-the-fly query embedding failures must degrade
        // to graph-based fallback instead of crashing FindSimilarConcepts.
        Ok(_) => match vector::generate_embedding(concept_name) {
            // T-12.3: RTM NFR-1 keeps invalid query embeddings out of vec0 distance queries.
            Ok(embedding) if embedding_is_valid_for_similarity_search(&embedding) => {
                vector::to_sqlite_vector_blob(&embedding)
            }
            Ok(_) => {
                return Err(MaenifoldError::InvalidArgument(
                    "unable to generate a valid embedding for conceptName".to_string(),
                ));
            }
            Err(_) => return Ok(None),
        },
        Err(error) if is_vec_embedding_unavailable(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut statement = connection.prepare(
        "SELECT concept_name, vec_distance_cosine(embedding, ?1) as distance \
         FROM vec_concepts \
         WHERE concept_name != ?2 \
           AND concept_name != '' \
           AND instr(concept_name, '[') = 0 \
           AND instr(concept_name, ']') = 0 \
         ORDER BY distance \
         LIMIT ?3",
    )?;
    // PARITY-S01: RTM FR-21.3 (RTM-109/RTM-112) — vec_distance_cosine() returns NULL
    // for malformed or zero embeddings; map to Option and filter below.
    let rows = statement.query_map(
        params![query_blob, concept_name, max_results as i64],
        |row| {
            let name: String = row.get(0)?;
            let distance: Option<f64> = row.get(1)?;
            match distance {
                Some(d) => Ok(Some((name, d))),
                None => Ok(None),
            }
        },
    )?;

    let mut results = Vec::new();
    for row in rows {
        let Some((neighbor, distance)) = row? else {
            continue;
        };
        if !distance.is_finite() {
            continue;
        }

        // T-QUAL-FSC2: RTM FR-7.4
        // Clamp negative-distance drift so semantic similarity never exceeds 1.000.
        let similarity = (1.0_f64 / (1.0_f64 + distance.max(0.0_f64))).clamp(0.0_f64, 1.0_f64);
        results.push(SimilarConceptMatch {
            concept: neighbor,
            distance,
            similarity,
        });
    }
    Ok(Some(results))
}

pub(crate) fn similarity_plateau_warning(
    query: &str,
    results: &[SimilarConceptMatch],
) -> Option<String> {
    if results.len() < 2 {
        return None;
    }

    let mean = results.iter().map(|result| result.distance).sum::<f64>() / results.len() as f64;
    let variance = results
        .iter()
        .map(|result| {
            let delta = result.distance - mean;
            delta * delta
        })
        .sum::<f64>()
        / results.len() as f64;

    (variance <= 1e-9).then(|| {
        format!(
            "similarity plateau detected for '{}'; results may be degraded by collapsed embedding distances.",
            query
        )
    })
}
