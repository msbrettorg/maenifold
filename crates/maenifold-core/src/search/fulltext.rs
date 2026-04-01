use crate::decay::compute_decay_weight_with_created;
use crate::error::Result;
use crate::search::api::SearchRequest;
use crate::search::shared::{
    build_search_response, fetch_text_results, retain_tagged_text_results, sort_results_by_score,
};
use crate::search::text::build_snippet;
use crate::tools::ToolContext;
use rusqlite::Connection;
use serde_json::{Value, json};

pub(crate) fn perform_fulltext_search(
    context: &ToolContext,
    connection: &Connection,
    request: &SearchRequest,
    query_terms: &[String],
    query_slug: &str,
) -> Result<String> {
    let mut text_results = fetch_text_results(connection, query_terms, request.folder_prefix())?;
    retain_tagged_text_results(context, &mut text_results, request.requested_tags());

    let max_score = text_results
        .iter()
        .map(|(_, score, _, _, _, _, _)| *score)
        .fold(0.0, f64::max);

    let mut results: Vec<Value> = if max_score > 0.0 {
        text_results
            .iter()
            .filter_map(
                |(path, text_score, title, content, created, last_accessed, status)| {
                    let normalized = text_score / max_score;
                    if normalized < request.min_score {
                        return None;
                    }
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
                        "score": normalized * decay_weight,
                        "snippet": build_snippet(content, query_terms, query_slug),
                    }))
                },
            )
            .collect()
    } else {
        Vec::new()
    };
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
        &["ReadMemory", "BuildContext"],
    ))
}
