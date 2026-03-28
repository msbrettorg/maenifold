use crate::error::Result;
use crate::markdown::normalize_concept;
use crate::repair::split_frontmatter_for_repair;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

// T-1.6: RTM NFR-1 - port CreateSmartPreview parity helper from C# GraphTools.cs.
pub fn create_smart_preview(content: &str, target_length: usize, tolerance: usize) -> String {
    let content_len = content.chars().count();
    if content_len <= target_length {
        return content.to_string();
    }

    let max_length = target_length.saturating_add(tolerance).min(content_len);
    let search_space = truncate_chars(content, max_length);
    let minimum_acceptable_boundary = target_length.saturating_sub(tolerance);

    if let Some(sentence_end) = find_last_sentence_boundary(search_space)
        && sentence_end >= minimum_acceptable_boundary
    {
        return truncate_chars(content, sentence_end + 1)
            .trim_end()
            .to_string();
    }

    if let Some(paragraph_end) = search_space.rfind("\n\n") {
        let paragraph_char_end = search_space[..paragraph_end].chars().count();
        if paragraph_char_end >= minimum_acceptable_boundary {
            return format!("{}...", search_space[..paragraph_end].trim_end());
        }
    }

    if let Some(word_end) = search_space.rfind(' ')
        && word_end > 0
    {
        return format!("{}...", search_space[..word_end].trim_end());
    }

    format!("{}...", truncate_chars(content, target_length))
}

fn truncate_chars(content: &str, char_count: usize) -> &str {
    if char_count == 0 {
        return "";
    }

    match content.char_indices().nth(char_count) {
        Some((index, _)) => &content[..index],
        None => content,
    }
}

fn find_last_sentence_boundary(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut last_index = None;

    for (index, ch) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let is_boundary = matches!(chars.get(index + 1), Some(' ' | '\n') | None);

        if is_boundary {
            last_index = Some(index);
        }
    }

    last_index
}

// T-13.6: RTM NFR-1 — parses source_files JSON into a JSON array for BuildContext output
pub(crate) fn parse_source_files_for_json(source_files: &str) -> Value {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(array)) => Value::Array(array),
        Ok(Value::Object(map)) => {
            Value::Array(map.keys().cloned().map(Value::String).collect::<Vec<_>>())
        }
        Ok(other) => other,
        Err(_) => Value::Array(
            source_files
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
                .collect(),
        ),
    }
}

// T-13.6: RTM NFR-1 — extracts file URI strings from source_files JSON
fn parse_source_file_uris(source_files: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(entries)) => entries
            .into_iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect(),
        // T-0.3: RTM-047 - Preserve source file URI extraction on older serde_json Map APIs.
        Ok(Value::Object(map)) => map.into_iter().map(|(key, _)| key).collect(),
        Ok(Value::String(uri)) => vec![uri],
        Ok(_) => Vec::new(),
        Err(_) => source_files
            .split(',')
            .map(str::trim)
            .filter(|uri| !uri.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

// T-13.6: RTM NFR-1 — loads content previews for source files keyed by URI
pub(crate) fn load_content_preview_map(
    connection: &Connection,
    focus_concept: &str,
    source_files: &str,
) -> Result<Value> {
    let mut previews = serde_json::Map::new();
    for uri in parse_source_file_uris(source_files) {
        let content = connection
            .query_row(
                "SELECT COALESCE(content, '') FROM file_content WHERE file_path = ?1",
                [uri.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(content) = content else {
            continue;
        };
        let preview = extract_section_with_concept(&content, focus_concept, 500);
        if !preview.is_empty() {
            previews.insert(uri, Value::String(preview));
        }
    }
    Ok(Value::Object(previews))
}

// T-1.6: RTM NFR-1 - ports C# ConceptGraphTests ExtractSectionWithConcept coverage for BuildContext previews.
pub fn extract_section_with_concept(
    content: &str,
    concept_name: &str,
    max_length: usize,
) -> String {
    let (_, body, _) = split_frontmatter_for_repair(content);
    let body = body.trim();
    if body.is_empty() || max_length == 0 {
        return String::new();
    }

    let normalized_concept = normalize_concept(concept_name);
    if normalized_concept.is_empty() {
        return truncate_preview(body, max_length);
    }

    let mut sections = Vec::new();
    let mut current_section = String::new();
    for line in body.lines() {
        if line.starts_with("## ") && !current_section.trim().is_empty() {
            sections.push(std::mem::take(&mut current_section));
        }
        current_section.push_str(line);
        current_section.push('\n');
    }
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    if let Some(section) = sections
        .iter()
        .find(|section| section_contains_concept(section, &normalized_concept))
    {
        return truncate_preview(section, max_length);
    }

    truncate_preview(body, max_length)
}

fn section_contains_concept(section: &str, normalized_concept: &str) -> bool {
    let section_lower = section.to_ascii_lowercase();
    if section_lower.contains(&format!("[[{normalized_concept}]]")) {
        return true;
    }

    let plain_concept = normalized_concept.replace('-', " ");
    if plain_concept.is_empty() {
        return false;
    }

    let pattern = format!(
        r"(?iu)(?:^|[^\p{{L}}\p{{N}}]){}(?:$|[^\p{{L}}\p{{N}}])",
        regex::escape(&plain_concept)
    );
    Regex::new(&pattern)
        .map(|regex| regex.is_match(section))
        .unwrap_or(false)
}

fn truncate_preview(content: &str, max_length: usize) -> String {
    let trimmed = content.trim();
    let char_count = trimmed.chars().count();
    if char_count <= max_length {
        return trimmed.to_string();
    }

    let truncated = trimmed
        .chars()
        .take(max_length)
        .collect::<String>()
        .trim_end()
        .to_string();
    format!("{truncated}...")
}
