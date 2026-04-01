use crate::error::Result;
use crate::search::display::file_matches_all_tags;
use crate::search::text::render_search_markdown;
use crate::tools::ToolContext;
use crate::tools::with_next_steps;
use crate::util::json::{OutputFormat, normalize_whole_floats, success};
use rusqlite::{Connection, params};
use serde_json::Value;

pub(crate) type TextResult = (
    String,
    f64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) fn fetch_text_results(
    connection: &Connection,
    query_terms: &[String],
    folder_prefix: Option<&str>,
) -> Result<Vec<TextResult>> {
    if query_terms.is_empty() {
        return Ok(Vec::new());
    }

    let match_expr = query_terms
        .iter()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ");

    let limit: i64 = 200;
    let mut statement = connection.prepare(
        "SELECT f.file_path, COALESCE(f.title, ''),
                snippet(file_search, 2, '', '', '...', 64) AS snippet,
                bm25(file_search, 1.0, 2.0) * -1 AS score,
                f.created, f.last_accessed, f.status
         FROM file_search
         JOIN file_content f ON f.rowid = file_search.rowid
         WHERE file_search MATCH ?1
         ORDER BY score DESC
         LIMIT ?2",
    )?;

    let rows = statement.query_map(params![match_expr, limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(3)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let mut text_results = Vec::new();
    for row in rows {
        let result = row?;
        if !path_in_scope(&result.0, folder_prefix) {
            continue;
        }
        text_results.push(result);
    }

    Ok(text_results)
}

pub(crate) fn retain_tagged_text_results(
    context: &ToolContext,
    text_results: &mut Vec<TextResult>,
    requested_tags: Option<&[String]>,
) {
    if let Some(tags) = requested_tags {
        text_results
            .retain(|(path, _, _, _, _, _, _)| path_matches_requested_tags(context, path, tags));
    }
}

pub(crate) fn retain_tagged_vector_results(
    context: &ToolContext,
    vector_results: &mut Option<Vec<(String, f64)>>,
    requested_tags: Option<&[String]>,
) {
    if let (Some(tags), Some(vec_results)) = (requested_tags, vector_results.as_mut()) {
        vec_results.retain(|(path, _)| path_matches_requested_tags(context, path, tags));
    }
}

pub(crate) fn build_search_response(
    context: &ToolContext,
    mut data: Value,
    next_steps: &[&str],
) -> String {
    normalize_whole_floats(&mut data);
    match context.output_format {
        OutputFormat::Json => success(data),
        OutputFormat::Markdown => with_next_steps(&render_search_markdown(&data), next_steps),
    }
}

pub(crate) fn sort_results_by_score(results: &mut [Value], score_key: &str) {
    results.sort_by(|a, b| {
        b[score_key]
            .as_f64()
            .partial_cmp(&a[score_key].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });
}

fn path_in_scope(path: &str, folder_prefix: Option<&str>) -> bool {
    match folder_prefix {
        Some(prefix) => {
            let scoped_prefix = format!("{prefix}/");
            path == prefix || path.starts_with(&scoped_prefix)
        }
        None => true,
    }
}

fn path_matches_requested_tags(context: &ToolContext, path: &str, tags: &[String]) -> bool {
    match file_matches_all_tags(context, path, tags) {
        Ok(matches) => matches,
        Err(error) => {
            eprintln!("[TAG FILTER] Error filtering '{}': {error}", path);
            false
        }
    }
}
