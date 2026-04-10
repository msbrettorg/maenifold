use crate::decay::compute_decay_weight_with_created;
use crate::error::Result;
use crate::search::api::SearchRequest;
use crate::search::shared::{
    TextResult, build_search_response, fetch_text_results, retain_tagged_text_results,
    retain_tagged_vector_results, sort_results_by_score,
};
use crate::search::text::build_snippet;
use crate::search::vector::{
    FileMetadata, cleanup_orphan_vec_memory_files, hydrate_vector_file_metadata,
    query_vec_memory_files,
};
use crate::tools::ToolContext;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn perform_hybrid_search(
    context: &ToolContext,
    connection: &Connection,
    request: &SearchRequest,
    query_terms: &[String],
    query_slug: &str,
) -> Result<String> {
    let mut text_results = fetch_text_results(connection, query_terms, request.folder_prefix())?;
    retain_tagged_text_results(context, &mut text_results, request.requested_tags());

    let mut vector_results = query_vec_memory_files(
        connection,
        &context.config.assets_path,
        &request.query,
        request.folder_prefix(),
        (request.page_size * 3).min(200),
    )?;
    retain_tagged_vector_results(context, &mut vector_results, request.requested_tags());

    let (rrf_scores, original_scores, file_metadata) = rank_hybrid_results(
        connection,
        &mut vector_results,
        &text_results,
        request.rrf_k.unwrap_or(60.0),
        request.text_weight.unwrap_or(1.0),
        request.semantic_weight.unwrap_or(1.0),
        request.semantic_min_score.unwrap_or(0.0),
    )?;

    let mut results = build_hybrid_results(
        context,
        &rrf_scores,
        &original_scores,
        &file_metadata,
        query_terms,
        query_slug,
        request.min_score,
    );
    sort_results_by_score(&mut results, "fusedScore");
    let (text_result_count, semantic_result_count) = count_ranked_results(&results);
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
        "textResultCount": text_result_count,
        "semanticResultCount": semantic_result_count,
        "results": paged_results,
    });
    Ok(build_search_response(
        context,
        data,
        &["ReadMemory", "BuildContext"],
    ))
}

type RankedHybridResults = (
    BTreeMap<String, f64>,
    BTreeMap<String, (f64, f64)>,
    BTreeMap<String, FileMetadata>,
);

fn rank_hybrid_results(
    connection: &Connection,
    vector_results: &mut Option<Vec<(String, f64)>>,
    text_results: &[TextResult],
    rrf_k: f64,
    text_weight: f64,
    semantic_weight: f64,
    semantic_min_score: f64,
) -> Result<RankedHybridResults> {
    let k = rrf_k;
    let mut rrf_scores = BTreeMap::new();
    let mut original_scores = BTreeMap::new();
    let mut file_metadata = BTreeMap::new();

    for (index, (path, text_score, title, content, created, last_accessed, status)) in
        text_results.iter().enumerate()
    {
        let rrf = text_weight * (1.0 / (k + index as f64 + 1.0));
        *rrf_scores.entry(path.clone()).or_insert(0.0) += rrf;
        original_scores.entry(path.clone()).or_insert((0.0, 0.0)).0 = (text_score / 100.0).min(1.0);
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

    if let Some(vec_results) = vector_results.as_mut() {
        if semantic_min_score > 0.0 {
            vec_results.retain(|(_, score)| *score >= semantic_min_score);
        }
        let missing_paths = vec_results
            .iter()
            .map(|(path, _)| path.clone())
            .filter(|path| !file_metadata.contains_key(path))
            .collect::<BTreeSet<_>>();
        let hydration = hydrate_vector_file_metadata(connection, &missing_paths)?;
        if !hydration.orphan_paths.is_empty() {
            cleanup_orphan_vec_memory_files(connection, &hydration.orphan_paths)?;
            vec_results.retain(|(path, _)| !hydration.orphan_paths.contains(path));
        }
        file_metadata.extend(hydration.metadata);

        for (index, (path, vec_score)) in vec_results.iter().enumerate() {
            let rrf = semantic_weight * (1.0 / (k + index as f64 + 1.0));
            *rrf_scores.entry(path.clone()).or_insert(0.0) += rrf;
            original_scores.entry(path.clone()).or_insert((0.0, 0.0)).1 = *vec_score;
        }
    }

    Ok((rrf_scores, original_scores, file_metadata))
}

#[allow(clippy::too_many_arguments)]
fn build_hybrid_results(
    context: &ToolContext,
    rrf_scores: &BTreeMap<String, f64>,
    original_scores: &BTreeMap<String, (f64, f64)>,
    file_metadata: &BTreeMap<String, FileMetadata>,
    query_terms: &[String],
    query_slug: &str,
    min_score: f64,
) -> Vec<Value> {
    rrf_scores
        .iter()
        .filter_map(|(path, rrf_score)| {
            let (text_score, semantic_score) = original_scores.get(path)?;
            let (title, content, created, last_accessed, status) = file_metadata.get(path)?;
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
                "snippet": build_snippet(content, query_terms, query_slug),
            }))
        })
        .filter(|value| value["fusedScore"].as_f64().unwrap_or(0.0) >= min_score)
        .collect()
}

fn count_ranked_results(results: &[Value]) -> (usize, usize) {
    let text = results
        .iter()
        .filter(|value| value["textScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();
    let semantic = results
        .iter()
        .filter(|value| value["semanticScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();
    (text, semantic)
}

#[cfg(test)]
mod tests {
    use super::rank_hybrid_results;
    use crate::search::shared::TextResult;
    use rusqlite::{Connection, params};

    fn setup_connection(paths: &[&str]) -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite");
        connection
            .execute(
                "CREATE TABLE file_content (
                    file_path TEXT PRIMARY KEY,
                    title TEXT,
                    content TEXT,
                    created TEXT,
                    last_accessed TEXT,
                    status TEXT
                )",
                [],
            )
            .expect("create file_content");
        for path in paths {
            connection
                .execute(
                    "INSERT INTO file_content (file_path, title, content, created, last_accessed, status)
                     VALUES (?1, ?2, ?3, NULL, NULL, NULL)",
                    params![path, format!("Title for {path}"), format!("Content for {path}")],
                )
                .expect("insert file_content row");
        }
        connection
    }

    fn text_result(path: &str, score: f64) -> TextResult {
        (
            path.to_string(),
            score,
            format!("Title for {path}"),
            format!("Content for {path}"),
            None,
            None,
            None,
        )
    }

    #[test]
    fn rank_hybrid_results_preserves_default_behavior() {
        let connection = setup_connection(&["memory://alpha", "memory://beta"]);
        let text_results = vec![
            text_result("memory://alpha", 80.0),
            text_result("memory://beta", 50.0),
        ];
        let mut vector_results = Some(vec![
            ("memory://alpha".to_string(), 0.9),
            ("memory://beta".to_string(), 0.4),
        ]);

        let (rrf_scores, original_scores, file_metadata) = rank_hybrid_results(
            &connection,
            &mut vector_results,
            &text_results,
            60.0,
            1.0,
            1.0,
            0.0,
        )
        .expect("rank hybrid results");

        assert_eq!(vector_results.as_ref().map(Vec::len), Some(2));
        assert!((rrf_scores["memory://alpha"] - (2.0 / 61.0)).abs() < 1e-12);
        assert!((rrf_scores["memory://beta"] - (2.0 / 62.0)).abs() < 1e-12);
        assert_eq!(original_scores["memory://alpha"], (0.8, 0.9));
        assert_eq!(original_scores["memory://beta"], (0.5, 0.4));
        assert!(file_metadata.contains_key("memory://alpha"));
        assert!(file_metadata.contains_key("memory://beta"));
    }

    #[test]
    fn rank_hybrid_results_applies_weights_and_semantic_min_score() {
        let connection = setup_connection(&["memory://alpha", "memory://beta"]);
        let text_results = vec![
            text_result("memory://alpha", 80.0),
            text_result("memory://beta", 50.0),
        ];
        let mut vector_results = Some(vec![
            ("memory://beta".to_string(), 0.2),
            ("memory://alpha".to_string(), 0.8),
        ]);

        let (rrf_scores, original_scores, _) = rank_hybrid_results(
            &connection,
            &mut vector_results,
            &text_results,
            10.0,
            2.0,
            3.0,
            0.5,
        )
        .expect("rank hybrid results");

        assert_eq!(
            vector_results,
            Some(vec![("memory://alpha".to_string(), 0.8)])
        );
        assert!((rrf_scores["memory://alpha"] - (5.0 / 11.0)).abs() < 1e-12);
        assert!((rrf_scores["memory://beta"] - (2.0 / 12.0)).abs() < 1e-12);
        assert_eq!(original_scores["memory://alpha"], (0.8, 0.8));
        assert_eq!(original_scores["memory://beta"], (0.5, 0.0));
    }
}
