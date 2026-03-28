use crate::error::Result;
use crate::search::display::file_matches_all_tags;
use crate::search::text::render_search_markdown;
use crate::tools::ToolContext;
use crate::tools::with_next_steps;
use crate::util::json::{OutputFormat, normalize_whole_floats, success};
use rusqlite::Connection;
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

    let mut text_results = Vec::new();
    for row in rows {
        let (path, title, content, created, last_accessed, status) = row?;
        if !path_in_scope(&path, folder_prefix) {
            continue;
        }

        let text_score = score_text_match(
            &title.to_ascii_lowercase(),
            &content.to_ascii_lowercase(),
            query_terms,
        );
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

fn score_text_match(title_lower: &str, content_lower: &str, query_terms: &[String]) -> f64 {
    if query_terms.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;
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
