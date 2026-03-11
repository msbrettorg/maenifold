use serde_json::Value;
use std::collections::HashSet;

pub(crate) fn slugify_query(query: &str) -> String {
    query
        .to_ascii_lowercase()
        .replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn informative_query_terms(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
        "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
        "these", "they", "this", "to", "was", "will", "with", "we", "you", "your", "our", "from",
    ];
    let stopwords = STOPWORDS.iter().copied().collect::<HashSet<_>>();
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .filter(|term| !stopwords.contains(term.as_str()))
        .collect()
}

pub(crate) fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

pub(crate) fn build_snippet(content: &str, query_terms: &[String], query_slug: &str) -> String {
    let lower = content.to_ascii_lowercase();
    let index = query_terms
        .iter()
        .find_map(|term| lower.find(term))
        .or_else(|| lower.find(query_slug))
        .unwrap_or(0);
    // T-12.7: RTM NFR-1 keeps the split [[search]] snippet behavior aligned with the stable facade contract.
    let start = index.saturating_sub(20);
    let end = (start + 200).min(content.len());
    content
        .get(start..end)
        .unwrap_or(content)
        .replace("\r\n", " ")
        .replace('\n', " ")
}

// T-13.7: RTM FR-21 — render search results in C#-parity markdown format with
// emoji headers, per-mode score display, and paged result count.
pub(crate) fn render_search_markdown(data: &Value) -> String {
    let mode = data.get("mode").and_then(Value::as_str).unwrap_or("Hybrid");
    let page = data.get("page").and_then(Value::as_u64).unwrap_or(1);
    let results = data.get("results").and_then(Value::as_array);
    let result_count = results.map_or(0, |r| r.len());

    let is_semantic = mode.eq_ignore_ascii_case("semantic");
    let is_fulltext =
        mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text");
    let is_hybrid = mode.eq_ignore_ascii_case("hybrid");

    // T-13.7: RTM FR-21 — emoji-prefixed bold headers matching C# SearchMemories output
    let heading = if is_semantic {
        "🧠 **Semantic Search** (concept similarity)"
    } else if is_fulltext {
        "📝 **Full-Text Search** (keyword matching)"
    } else if is_hybrid {
        "🔍 **Hybrid Search** (combining text + semantic similarity)"
    } else {
        "🔍 **Search Results**"
    };

    let mut output = format!("{heading}\nFound {result_count} matches (page {page}):\n");

    // T-13.7: RTM FR-21 — Hybrid mode adds text/semantic sub-counts below the header
    if is_hybrid {
        let text_count = data
            .get("textResultCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let semantic_count = data
            .get("semanticResultCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        output.push_str(&format!(
            "Text results: {text_count}, Semantic results: {semantic_count}\n"
        ));
    }

    if let Some(results) = results {
        for result in results {
            let title = result
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let path = result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let snippet = result
                .get("snippet")
                .and_then(Value::as_str)
                .unwrap_or_default();

            // T-13.7: RTM FR-21 — 📄 bold title + URI line
            output.push_str(&format!("\n📄 **{title}** ({path})\n"));

            // T-13.7: RTM FR-21 — per-mode score lines with appropriate precision
            if is_fulltext {
                let score = result
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                output.push_str(&format!("   📊 Text relevance: {score:.2}\n"));
            } else if is_hybrid {
                let fused = result
                    .get("fusedScore")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let text = result
                    .get("textScore")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let semantic = result
                    .get("semanticScore")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                output.push_str(&format!(
                    "   📊 Scores - Fused: {fused:.3} | Text: {text:.3} | Semantic: {semantic:.3}\n"
                ));
            } else if is_semantic {
                let score = result
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                output.push_str(&format!("   🎯 Semantic similarity: {score:.3}\n"));
            }

            output.push_str(&format!("   📝 {snippet}\n"));
        }
    }

    output.trim_end().to_string()
}
