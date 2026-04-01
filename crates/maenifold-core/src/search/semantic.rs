use crate::decay::compute_decay_weight_with_created;
use crate::error::Result;
use crate::search::api::SearchRequest;
use crate::search::shared::{
    build_search_response, retain_tagged_vector_results, sort_results_by_score,
};
use crate::search::text::build_snippet;
use crate::search::vector::{
    cleanup_orphan_vec_memory_files, hydrate_vector_file_metadata, query_vec_memory_files,
};
use crate::tools::ToolContext;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) fn perform_semantic_search(
    context: &ToolContext,
    connection: &Connection,
    request: &SearchRequest,
    query_terms: &[String],
    query_slug: &str,
) -> Result<String> {
    let mut vector_results = query_vec_memory_files(
        connection,
        &context.config.assets_path,
        &request.query,
        request.folder_prefix(),
        (request.page_size * 2).min(200),
    )?;
    retain_tagged_vector_results(context, &mut vector_results, request.requested_tags());

    let mut vec_results = vector_results.unwrap_or_default();
    let candidate_paths = vec_results
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let hydration = hydrate_vector_file_metadata(connection, &candidate_paths)?;

    if !hydration.orphan_paths.is_empty() {
        cleanup_orphan_vec_memory_files(connection, &hydration.orphan_paths)?;
        vec_results.retain(|(path, _)| !hydration.orphan_paths.contains(path));
    }

    let mut results = build_semantic_results(
        context,
        &vec_results,
        &hydration.metadata,
        query_terms,
        query_slug,
        request.min_score,
    );
    sort_results_by_score(&mut results, "score");

    let total_count = results.len();
    let start = request.page.saturating_sub(1) * request.page_size;
    let paged_results = results
        .into_iter()
        .skip(start)
        .take(request.page_size)
        .collect::<Vec<_>>();

    let data = json!({
        "mode": request.mode.as_str(),
        "query": request.query.as_str(),
        "totalCount": total_count,
        "page": request.page,
        "pageSize": request.page_size,
        "results": paged_results,
    });
    Ok(build_search_response(
        context,
        data,
        &["ReadMemory", "BuildContext", "FindSimilarConcepts"],
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_semantic_results(
    context: &ToolContext,
    vec_results: &[(String, f64)],
    metadata: &std::collections::BTreeMap<String, crate::search::vector::FileMetadata>,
    query_terms: &[String],
    query_slug: &str,
    min_score: f64,
) -> Vec<Value> {
    vec_results
        .iter()
        .filter_map(|(path, score)| {
            let (title, content, created, last_accessed, status) = metadata.get(path)?;
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
        .collect()
}
