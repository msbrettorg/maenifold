mod support;
mod validation;
mod vector_query;

use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::normalize_concept;
use crate::tools::ToolContext;
use crate::tools::with_next_steps;
use rusqlite::params;
use serde_json::Value;
use std::cmp::Ordering;

use self::support::{concept_decay_weight, load_help_doc, with_similarity_next_steps};
use self::validation::{
    contains_bracket_like_delimiter, is_low_information_find_similar_query,
    normalize_find_similar_validation_input,
};
use self::vector_query::{query_vec_similar_concepts, similarity_plateau_warning};

pub fn find_similar_concepts(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        return Ok(
            load_help_doc(&context.config.assets_path, "FindSimilarConcepts").unwrap_or_else(
                || "ERROR: Help file not found for FindSimilarConcepts".to_string(),
            ),
        );
    }

    let concept_name = payload
        .get("conceptName")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("conceptName"))?
        .trim();
    if concept_name.is_empty() {
        return Err(MaenifoldError::Message("conceptName parameter is required and cannot be empty".to_string()));
    }
    if concept_name.len() > 256 {
        return Err(MaenifoldError::Message("conceptName must be 256 characters or fewer".to_string()));
    }

    let original_concept_name = concept_name.to_string();
    let low_info_query = concept_name.chars().count() < 2;
    let validation_input = normalize_find_similar_validation_input(concept_name);
    if validation_input.contains('[') || validation_input.contains(']') {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' contains bracket characters (or Unicode confusables). Remove brackets and try again."
        )));
    }
    if contains_bracket_like_delimiter(&validation_input) {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' contains bracket-like delimiter characters. Remove them and try again."
        )));
    }
    if is_low_information_find_similar_query(&validation_input) {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' is too low-information to embed reliably. Provide a few letters or words."
        )));
    }

    let max_results = payload
        .get("maxResults")
        .and_then(Value::as_u64)
        .unwrap_or(10);
    if !(1..=50).contains(&max_results) {
        return Err(MaenifoldError::Message("maxResults must be between 1 and 50".to_string()));
    }

    let concept_name = normalize_concept(concept_name);
    if concept_name.is_empty() {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{}' normalizes to empty. Provide a few letters or words.",
            payload
                .get("conceptName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
        )));
    }

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;

    if let Some(vector_results) = query_vec_similar_concepts(
        &connection,
        &context.config.assets_path,
        &concept_name,
        max_results as usize,
    )? {
        // T-9.2 / T-3.1: RTM FR-8, FR-6 — when vec0 tables exist but are empty
        // (e.g. after ensure_vec0_tables creates them but before sync populates
        // embeddings), fall through to graph-based search rather than returning
        // "no results". Non-empty results still take the fast vector path.
        if !vector_results.is_empty() {
            let plateau_warning =
                similarity_plateau_warning(&original_concept_name, &vector_results);
            let mut output = format_ranked_results(
                &connection,
                &context.config,
                &concept_name,
                vector_results
                    .into_iter()
                    .map(|result| (result.concept, result.similarity))
                    .collect(),
            );
            if let Some(warning) = plateau_warning {
                // T-12.3: RTM NFR-1 keeps the vec0 plateau warning explicit without altering result output.
                output.push_str(&format!("\n⚠️ {warning}\n"));
            }
            append_short_query_warning(&mut output, &original_concept_name, low_info_query);
            // T-13.5: RTM FR-8 — use centralized with_next_steps helper
            output = with_next_steps(&output, &["BuildContext", "SearchMemories"]);
            return Ok(output);
        }
    }

    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0) FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1 ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC LIMIT ?2",
    )?;
    let rows = statement.query_map(params![concept_name, max_results as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
        ))
    })?;

    let mut neighbors = Vec::new();
    let mut max_count = 0.0_f64;
    for row in rows {
        let (concept_a, concept_b, count) = row?;
        let neighbor = if concept_a.eq_ignore_ascii_case(&concept_name) {
            concept_b
        } else {
            concept_a
        };
        if neighbor.eq_ignore_ascii_case(&concept_name) {
            continue;
        }
        max_count = max_count.max(count);
        neighbors.push((neighbor, count));
    }

    if neighbors.is_empty() {
        return Ok(with_similarity_next_steps(format!(
            "No similar concepts found for '{}'. Run Sync to ensure concept embeddings are generated.",
            concept_name
        )));
    }

    let scored_neighbors = neighbors
        .into_iter()
        .map(|(neighbor, count)| {
            let similarity = if max_count <= 0.0 {
                0.0
            } else {
                count / max_count
            };
            (neighbor, similarity)
        })
        .collect();
    let mut output = format_ranked_results(
        &connection,
        &context.config,
        &concept_name,
        scored_neighbors,
    );
    append_short_query_warning(&mut output, &original_concept_name, low_info_query);
    // T-13.5: RTM FR-8 — use centralized with_next_steps helper
    output = with_next_steps(&output, &["BuildContext", "SearchMemories"]);
    Ok(output)
}

fn format_ranked_results(
    connection: &rusqlite::Connection,
    config: &Config,
    concept_name: &str,
    candidates: Vec<(String, f64)>,
) -> String {
    // T-12.3: RTM NFR-1 preserves ranking/output behavior while delegating helpers across the new similar/ module.
    let mut ranked_results = candidates
        .into_iter()
        .map(|(neighbor, similarity)| {
            let decay_weight = concept_decay_weight(connection, config, &neighbor).unwrap_or(1.0);
            let weighted_similarity = similarity * decay_weight;
            (neighbor, similarity, weighted_similarity)
        })
        .collect::<Vec<_>>();
    ranked_results.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut output = format!(
        "Similar concepts to '{}' (by semantic similarity):\n\n",
        concept_name
    );
    // PARITY-F01: RTM NFR-1 — display weighted_similarity (similarity * decay_weight)
    // to match C# VectorSearchTools.cs which displays decayedSimilarity, not raw similarity.
    for (neighbor, _, weighted_similarity) in ranked_results {
        output.push_str(&format!(
            "  • {} (similarity: {:.3})\n",
            neighbor, weighted_similarity
        ));
    }
    output
}

fn append_short_query_warning(
    output: &mut String,
    original_concept_name: &str,
    low_info_query: bool,
) {
    if low_info_query {
        // T-12.3: RTM NFR-1 keeps the short-query warning aligned across vec0 and graph fallback paths.
        output.push_str(&format!(
            "\n⚠️ conceptName '{}' is very short; similarity results may be noisy.\n",
            original_concept_name
        ));
    }
}
