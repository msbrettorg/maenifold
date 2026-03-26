use crate::db::{self, GraphDatabase};
use crate::decay::compute_decay_weight_with_created;
use crate::error::{MaenifoldError, Result};
use crate::search::display::{file_matches_all_tags, load_help_doc};
use crate::search::text::{
    build_snippet, informative_query_terms, render_search_markdown, slugify_query, tokenize_query,
};
use crate::search::vector::{
    FileMetadata, cleanup_orphan_vec_memory_files, hydrate_vector_file_metadata,
    query_vec_memory_files,
};
use crate::tools::ToolContext;
// T-13.5: RTM FR-8 — centralized response helpers for next-steps / hints
use crate::tools::{with_hint, with_next_steps};
use crate::util::json::{OutputFormat, normalize_whole_floats, success};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

// T-13.1: RTM FR-6 — (path, text_score, title, content, created, last_accessed, status)
type TextResult = (
    String,
    f64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

// T-12.7: RTM NFR-1 keeps the caller-facing [[search]] facade stable while delegating orchestration into split submodules.
pub fn search_memories(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        return Ok(load_help_doc(&context.config.assets_path, "SearchMemories")
            .unwrap_or_else(|| "ERROR: Help file not found for SearchMemories".to_string()));
    }

    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("query"))?;
    if informative_query_terms(query).is_empty() {
        // T-13.5: RTM FR-8 — match C# MemorySearchTools.QueryLacksInformativeTermsResponse()
        return Ok(with_hint(
            "ERROR: Query contains no informative keywords. Try adding specific terms (e.g. \"machine learning\", \"NLP\").",
            "SearchMemories",
            "Provide at least one non-stopword keyword",
        ));
    }
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("Hybrid");
    let page_size = payload
        .get("pageSize")
        .and_then(Value::as_u64)
        .unwrap_or(10) as usize;
    let page = payload.get("page").and_then(Value::as_u64).unwrap_or(1) as usize;
    let min_score = payload
        .get("minScore")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let folder_prefix = payload
        .get("folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(|folder| format!("memory://{}", folder.trim_matches('/')));
    let requested_tags = payload
        .get("tags")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(|tag| tag.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|tags| !tags.is_empty());

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;

    // T-13.1: RTM FR-6 — dispatch to mode-specific search paths
    if mode.eq_ignore_ascii_case("semantic") {
        let qt = tokenize_query(query);
        let qs = slugify_query(query);
        return perform_semantic_search(
            context,
            &connection,
            query,
            &qt,
            &qs,
            folder_prefix.as_deref(),
            requested_tags.as_deref(),
            page_size,
            page,
            min_score,
            mode,
        );
    }
    if mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text") {
        let qt = tokenize_query(query);
        let qs = slugify_query(query);
        return perform_fulltext_search(
            context,
            &connection,
            query,
            &qt,
            &qs,
            folder_prefix.as_deref(),
            requested_tags.as_deref(),
            page_size,
            page,
            min_score,
            mode,
        );
    }

    let mut statement = connection.prepare(
        "SELECT file_path, COALESCE(title, ''), COALESCE(content, ''), created, last_accessed, status FROM file_content",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let query_slug = slugify_query(query);
    let query_terms = tokenize_query(query);
    let mut text_results: Vec<TextResult> = Vec::new();

    for row in rows {
        let (path, title, content, created, last_accessed, status) = row?;
        if let Some(prefix) = folder_prefix.as_deref() {
            let scoped_prefix = format!("{prefix}/");
            if path != prefix && !path.starts_with(&scoped_prefix) {
                continue;
            }
        }
        let title_lower = title.to_ascii_lowercase();
        let content_lower = content.to_ascii_lowercase();

        let text_score = if query_terms.is_empty() {
            0.0
        } else {
            let mut score = 0.0f64;
            for term in &query_terms {
                let term_str = term.as_str();
                let mut search_start = 0;
                while let Some(pos) = content_lower[search_start..].find(term_str) {
                    score += 1.0;
                    search_start += pos + term_str.len();
                }
                if !title_lower.is_empty() && title_lower.contains(term_str) {
                    score += 5.0;
                }
            }
            score
        };

        if text_score > 0.0 {
            text_results.push((
                path,
                text_score,
                title,
                content,
                created,
                last_accessed,
                status,
            ));
        }
    }

    text_results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    if let Some(tags) = requested_tags.as_deref() {
        text_results.retain(|(path, _, _, _, _, _, _)| {
            match file_matches_all_tags(context, path, tags) {
                Ok(matches) => matches,
                Err(error) => {
                    eprintln!("[TAG FILTER] Error filtering '{}': {error}", path);
                    false
                }
            }
        });
    }

    // T-13.2: RTM FR-6 — C# uses PreFusionMultiplier=3 for hybrid candidate fetching
    let max_candidates = (page_size * 3).min(200);
    let mut vector_results = query_vec_memory_files(
        &connection,
        &context.config.assets_path,
        query,
        folder_prefix.as_deref(),
        max_candidates,
    )?;

    if let Some(ref mut vec_results) = vector_results {
        if let Some(tags) = requested_tags.as_deref() {
            vec_results.retain(|(path, _)| match file_matches_all_tags(context, path, tags) {
                Ok(matches) => matches,
                Err(error) => {
                    eprintln!("[TAG FILTER] Error filtering '{}': {error}", path);
                    false
                }
            });
        }
    }

    let k = 60.0f64;
    let mut rrf_scores: BTreeMap<String, f64> = BTreeMap::new();
    let mut original_scores: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut file_metadata: BTreeMap<String, FileMetadata> = BTreeMap::new();

    for (index, (path, text_score, title, content, created, last_accessed, status)) in
        text_results.iter().enumerate()
    {
        let rrf = 1.0 / (k + index as f64 + 1.0);
        *rrf_scores.entry(path.clone()).or_insert(0.0) += rrf;
        let entry = original_scores.entry(path.clone()).or_insert((0.0, 0.0));
        entry.0 = (text_score / 100.0).min(1.0);
        file_metadata.entry(path.clone()).or_insert_with(|| {
            (
                title.clone(),
                content.clone(),
                created.clone(),
                last_accessed.clone(),
                status.clone(),
            )
        });
    }

    if let Some(ref mut vec_results) = vector_results {
        let missing_paths = vec_results
            .iter()
            .map(|(path, _)| path.clone())
            .filter(|path| !file_metadata.contains_key(path))
            .collect::<BTreeSet<_>>();
        let hydration = hydrate_vector_file_metadata(&connection, &missing_paths)?;
        if !hydration.orphan_paths.is_empty() {
            cleanup_orphan_vec_memory_files(&connection, &hydration.orphan_paths)?;
            vec_results.retain(|(path, _)| !hydration.orphan_paths.contains(path));
        }
        file_metadata.extend(hydration.metadata);

        for (index, (path, vec_score)) in vec_results.iter().enumerate() {
            let rrf = 1.0 / (k + index as f64 + 1.0);
            *rrf_scores.entry(path.clone()).or_insert(0.0) += rrf;
            let entry = original_scores.entry(path.clone()).or_insert((0.0, 0.0));
            entry.1 = *vec_score;
        }
    }

    let mut results: Vec<Value> = rrf_scores
        .iter()
        .filter_map(|(path, rrf_score)| {
            let (text_score, semantic_score) = original_scores.get(path)?;
            let (title, content, created, last_accessed, status) = file_metadata.get(path)?;
            // T-14.1: RTM FR-21 threads DB `created` into decay scoring so
            // search ranking matches the C# `lastAccessed ?? created` fallback.
            let decay_weight = compute_decay_weight_with_created(
                path,
                last_accessed.as_deref(),
                created.as_deref(),
                &context.config,
                status.as_deref(),
            );
            Some(json!({
                "title": title,
                "path": path,
                "decayWeight": decay_weight,
                "fusedScore": rrf_score * decay_weight,
                "textScore": *text_score,
                "semanticScore": *semantic_score,
                "snippet": build_snippet(content, &query_terms, &query_slug),
            }))
        })
        .collect();

    results.sort_by(|a, b| {
        b["fusedScore"]
            .as_f64()
            .partial_cmp(&a["fusedScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });

    let mode_score_key = if mode.eq_ignore_ascii_case("semantic") {
        "semanticScore"
    } else if mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text") {
        "textScore"
    } else {
        "fusedScore"
    };

    let results = results
        .into_iter()
        .filter(|value| value[mode_score_key].as_f64().unwrap_or(0.0) >= min_score)
        .collect::<Vec<_>>();

    let text_result_count = results
        .iter()
        .filter(|value| value["textScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();
    let semantic_result_count = results
        .iter()
        .filter(|value| value["semanticScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();

    let total_count = results.len();
    let start = page.saturating_sub(1) * page_size;
    let paged_results = results
        .into_iter()
        .skip(start)
        .take(page_size)
        .collect::<Vec<_>>();

    // T-16.5: RTM NFR-1 — normalize whole-number floats to match C# System.Text.Json output
    let mut data = json!({
        "mode": mode,
        "query": query,
        "totalCount": total_count,
        "page": page,
        "pageSize": page_size,
        "textResultCount": text_result_count,
        "semanticResultCount": semantic_result_count,
        "results": paged_results,
    });
    normalize_whole_floats(&mut data);

    Ok(match context.output_format {
        OutputFormat::Json => success(data),
        // T-13.5: RTM FR-8 — C# Hybrid search appends WithNextSteps("ReadMemory", "BuildContext")
        OutputFormat::Markdown => with_next_steps(
            &render_search_markdown(&data),
            &["ReadMemory", "BuildContext"],
        ),
    })
}

// T-13.1: RTM FR-6 — Semantic mode: vector-only search with decay weighting, single `score` field.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn perform_semantic_search(
    context: &ToolContext,
    connection: &Connection,
    query: &str,
    query_terms: &[String],
    query_slug: &str,
    folder_prefix: Option<&str>,
    requested_tags: Option<&[String]>,
    page_size: usize,
    page: usize,
    min_score: f64,
    mode: &str,
) -> Result<String> {
    // T-13.2: RTM FR-6 — C# PerformSemanticSearch uses pageSize * 2 for single-mode candidate limit
    let max_candidates = (page_size * 2).min(200);
    let mut vector_results = query_vec_memory_files(
        connection,
        &context.config.assets_path,
        query,
        folder_prefix,
        max_candidates,
    )?;

    if let Some(ref mut vec_results) = vector_results
        && let Some(tags) = requested_tags
    {
        vec_results.retain(|(path, _)| match file_matches_all_tags(context, path, tags) {
            Ok(matches) => matches,
            Err(error) => {
                eprintln!("[TAG FILTER] Error filtering '{}': {error}", path);
                false
            }
        });
    }

    let mut vec_results = vector_results.unwrap_or_default();

    // Hydrate metadata for all vector result paths
    let candidate_paths: BTreeSet<String> = vec_results.iter().map(|(p, _)| p.clone()).collect();
    let hydration = hydrate_vector_file_metadata(connection, &candidate_paths)?;

    // Clean up orphaned vec entries that have no corresponding file_content row
    if !hydration.orphan_paths.is_empty() {
        cleanup_orphan_vec_memory_files(connection, &hydration.orphan_paths)?;
        vec_results.retain(|(path, _)| !hydration.orphan_paths.contains(path));
    }

    // Build results: decay weight * raw semantic similarity score
    let mut results: Vec<Value> = vec_results
        .iter()
        .filter_map(|(path, score)| {
            let (title, content, created, last_accessed, status) = hydration.metadata.get(path)?;
            let decay_weight = compute_decay_weight_with_created(
                path,
                last_accessed.as_deref(),
                created.as_deref(),
                &context.config,
                status.as_deref(),
            );
            let decayed_score = score * decay_weight;
            if decayed_score < min_score {
                return None;
            }
            Some(json!({
                "title": title,
                "path": path,
                "decayWeight": decay_weight,
                "score": decayed_score,
                "snippet": build_snippet(content, query_terms, query_slug),
            }))
        })
        .collect();

    results.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .partial_cmp(&a["score"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });

    let total_count = results.len();
    let start = page.saturating_sub(1) * page_size;
    let paged_results: Vec<_> = results.into_iter().skip(start).take(page_size).collect();

    // T-16.5: RTM NFR-1 — normalize whole-number floats to match C# System.Text.Json output
    let mut data = json!({
        "mode": mode,
        "query": query,
        "totalCount": total_count,
        "page": page,
        "pageSize": page_size,
        "results": paged_results,
    });
    normalize_whole_floats(&mut data);

    Ok(match context.output_format {
        OutputFormat::Json => success(data),
        // T-13.5: RTM FR-8 — C# Semantic search appends WithNextSteps("ReadMemory", "BuildContext", "FindSimilarConcepts")
        OutputFormat::Markdown => with_next_steps(
            &render_search_markdown(&data),
            &["ReadMemory", "BuildContext", "FindSimilarConcepts"],
        ),
    })
}

// T-13.1: RTM FR-6 — FullText mode: text-only search with max-normalization and decay weighting.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn perform_fulltext_search(
    context: &ToolContext,
    connection: &Connection,
    query: &str,
    query_terms: &[String],
    query_slug: &str,
    folder_prefix: Option<&str>,
    requested_tags: Option<&[String]>,
    page_size: usize,
    page: usize,
    min_score: f64,
    mode: &str,
) -> Result<String> {
    let mut statement = connection.prepare(
        "SELECT file_path, COALESCE(title, ''), COALESCE(content, ''), created, last_accessed, status FROM file_content",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let mut text_results: Vec<TextResult> = Vec::new();

    for row in rows {
        let (path, title, content, created, last_accessed, status) = row?;
        if let Some(prefix) = folder_prefix {
            let scoped_prefix = format!("{prefix}/");
            if path != prefix && !path.starts_with(&scoped_prefix) {
                continue;
            }
        }
        let title_lower = title.to_ascii_lowercase();
        let content_lower = content.to_ascii_lowercase();

        let text_score = if query_terms.is_empty() {
            0.0
        } else {
            let mut score = 0.0f64;
            for term in query_terms {
                let term_str = term.as_str();
                let mut search_start = 0;
                while let Some(pos) = content_lower[search_start..].find(term_str) {
                    score += 1.0;
                    search_start += pos + term_str.len();
                }
                if !title_lower.is_empty() && title_lower.contains(term_str) {
                    score += 5.0;
                }
            }
            score
        };

        if text_score > 0.0 {
            text_results.push((
                path,
                text_score,
                title,
                content,
                created,
                last_accessed,
                status,
            ));
        }
    }

    text_results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    if let Some(tags) = requested_tags {
        text_results.retain(|(path, _, _, _, _, _, _)| {
            match file_matches_all_tags(context, path, tags) {
                Ok(matches) => matches,
                Err(error) => {
                    eprintln!("[TAG FILTER] Error filtering '{}': {error}", path);
                    false
                }
            }
        });
    }

    // T-13.1/T-13.3: RTM FR-6 — max-normalization: divide each score by the maximum score
    // so that the top result gets score=1.0 before decay is applied.
    let max_score = text_results
        .iter()
        .map(|(_, s, _, _, _, _, _)| *s)
        .fold(0.0f64, f64::max);

    // T-13.3: RTM FR-6 — C# applies minScore on the normalized (pre-decay) score,
    // then multiplies by decay weight afterward. Match that ordering exactly.
    let mut results: Vec<Value> = if max_score > 0.0 {
        text_results
            .iter()
            .filter_map(
                |(path, text_score, title, content, created, last_accessed, status)| {
                    let normalized = text_score / max_score;
                    if normalized < min_score {
                        return None;
                    }
                    let decay_weight = compute_decay_weight_with_created(
                        path,
                        last_accessed.as_deref(),
                        created.as_deref(),
                        &context.config,
                        status.as_deref(),
                    );
                    let decayed_score = normalized * decay_weight;
                    Some(json!({
                        "title": title,
                        "path": path,
                        "decayWeight": decay_weight,
                        "score": decayed_score,
                        "snippet": build_snippet(content, query_terms, query_slug),
                    }))
                },
            )
            .collect()
    } else {
        Vec::new()
    };

    results.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .partial_cmp(&a["score"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });

    let total_count = results.len();
    let start = page.saturating_sub(1) * page_size;
    let paged_results: Vec<_> = results.into_iter().skip(start).take(page_size).collect();

    // T-16.5: RTM NFR-1 — normalize whole-number floats to match C# System.Text.Json output
    let mut data = json!({
        "mode": mode,
        "query": query,
        "totalCount": total_count,
        "page": page,
        "pageSize": page_size,
        "results": paged_results,
    });
    normalize_whole_floats(&mut data);

    Ok(match context.output_format {
        OutputFormat::Json => success(data),
        // T-13.5: RTM FR-8 — C# FullText search appends WithNextSteps("ReadMemory", "BuildContext")
        OutputFormat::Markdown => with_next_steps(
            &render_search_markdown(&data),
            &["ReadMemory", "BuildContext"],
        ),
    })
}
