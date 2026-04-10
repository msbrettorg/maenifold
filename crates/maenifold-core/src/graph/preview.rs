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

fn find_next_sentence_boundary(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();

    for (index, ch) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let is_boundary = matches!(chars.get(index + 1), Some(' ' | '\n') | None);
        if is_boundary {
            return Some(index);
        }
    }

    None
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

    let matcher = ConceptMatcher::new(&normalized_concept);

    let sections = collect_sections(body);
    for allow_heading_matches in [false, true] {
        for section in &sections {
            let section_content = &body[section.start..section.end];
            if let Some(concept_match) =
                find_concept_match(section_content, &matcher, allow_heading_matches)
            {
                return centered_preview(section_content, concept_match, max_length);
            }
        }
    }

    truncate_preview(body, max_length)
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

#[derive(Clone, Copy)]
struct SectionRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct MatchSpan {
    start: usize,
    end: usize,
}

struct ConceptMatcher {
    wikilink_regexes: Vec<Regex>,
    plain_regex: Option<Regex>,
}

impl ConceptMatcher {
    fn new(normalized_concept: &str) -> Self {
        let plain_concepts = [
            normalized_concept.replace('-', " "),
            normalized_concept.to_string(),
            normalized_concept.replace('-', "_"),
        ]
        .into_iter()
        .fold(Vec::<String>::new(), |mut variants, variant| {
            if !variant.is_empty() && !variants.contains(&variant) {
                variants.push(variant);
            }
            variants
        });
        let plain_regex = if plain_concepts.is_empty() {
            None
        } else {
            let alternation = plain_concepts
                .iter()
                .map(|variant| regex::escape(variant))
                .collect::<Vec<_>>()
                .join("|");
            let pattern = format!(
                r"(?iu)(?:^|[^\p{{L}}\p{{N}}_-])({})(?:$|[^\p{{L}}\p{{N}}_-])",
                alternation
            );
            Regex::new(&pattern).ok()
        };

        let mut wikilink_variants = Vec::new();
        for variant in [
            normalized_concept.to_string(),
            normalized_concept.replace('-', "_"),
            normalized_concept.replace('-', " "),
        ] {
            let wikilink = format!("[[{variant}]]");
            if !wikilink_variants.contains(&wikilink) {
                wikilink_variants.push(wikilink);
            }
        }

        let wikilink_regexes = wikilink_variants
            .iter()
            .filter_map(|wikilink| {
                let pattern = format!(r"(?i){}", regex::escape(wikilink));
                Regex::new(&pattern).ok()
            })
            .collect();

        Self {
            wikilink_regexes,
            plain_regex,
        }
    }

    fn find_first(&self, text: &str) -> Option<MatchSpan> {
        let wikilink_match = self
            .wikilink_regexes
            .iter()
            .filter_map(|regex| {
                regex.find(text).map(|matched| MatchSpan {
                    start: matched.start(),
                    end: matched.end(),
                })
            })
            .min_by_key(|found| found.start);

        let plain_match = self.plain_regex.as_ref().and_then(|regex| {
            regex.captures(text).and_then(|captures| {
                captures.get(1).map(|matched| MatchSpan {
                    start: matched.start(),
                    end: matched.end(),
                })
            })
        });

        match (wikilink_match, plain_match) {
            (Some(left), Some(right)) => Some(if left.start <= right.start {
                left
            } else {
                right
            }),
            (Some(found), None) | (None, Some(found)) => Some(found),
            (None, None) => None,
        }
    }
}

fn collect_sections(body: &str) -> Vec<SectionRange> {
    let mut sections = Vec::new();
    let mut current_start = 0;
    let mut current_has_content = false;
    let mut offset = 0;

    for line in body.split_inclusive('\n') {
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        if line_content.starts_with("## ") && current_has_content {
            if !body[current_start..offset].trim().is_empty() {
                sections.push(SectionRange {
                    start: current_start,
                    end: offset,
                });
            }
            current_start = offset;
        }

        if !line_content.trim().is_empty() {
            current_has_content = true;
        }

        offset += line.len();
    }

    if !body[current_start..].trim().is_empty() {
        sections.push(SectionRange {
            start: current_start,
            end: body.len(),
        });
    }

    sections
}

fn find_concept_match(
    text: &str,
    matcher: &ConceptMatcher,
    allow_heading_matches: bool,
) -> Option<MatchSpan> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        let is_heading = line_content.trim_start().starts_with('#');
        if (allow_heading_matches || !is_heading)
            && let Some(concept_match) = matcher.find_first(line_content)
        {
            return Some(MatchSpan {
                start: offset + concept_match.start,
                end: offset + concept_match.end,
            });
        }

        offset += line.len();
    }

    None
}

fn centered_preview(content: &str, concept_match: MatchSpan, max_length: usize) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() || max_length == 0 {
        return String::new();
    }

    let total_chars = trimmed.chars().count();
    if total_chars <= max_length {
        return trimmed.to_string();
    }

    let trimmed_start = content.find(trimmed).unwrap_or(0);
    let trimmed_match = MatchSpan {
        start: concept_match.start.saturating_sub(trimmed_start),
        end: concept_match.end.saturating_sub(trimmed_start),
    };

    if trimmed_match.end > trimmed.len() || trimmed_match.start >= trimmed_match.end {
        return truncate_preview(trimmed, max_length);
    }

    if let Some(preview) = paragraph_preview(trimmed, trimmed_match, max_length) {
        return preview;
    }

    if let Some(preview) = sentence_preview(trimmed, trimmed_match, max_length) {
        return preview;
    }

    let match_start_char = byte_to_char_index(trimmed, trimmed_match.start);
    let match_end_char = byte_to_char_index(trimmed, trimmed_match.end);
    let available_chars = max_length.min(total_chars);
    let match_center = (match_start_char + match_end_char) / 2;

    let mut start_char = match_center.saturating_sub(available_chars / 2);
    let mut end_char = (start_char + available_chars).min(total_chars);
    if end_char - start_char < available_chars {
        start_char = end_char.saturating_sub(available_chars);
    }

    if start_char > match_start_char {
        start_char = match_start_char;
        end_char = (start_char + available_chars).min(total_chars);
    }

    if end_char < match_end_char {
        end_char = match_end_char;
        start_char = end_char.saturating_sub(available_chars);
    }

    render_window(trimmed, start_char, end_char, max_length)
}

fn paragraph_preview(content: &str, concept_match: MatchSpan, max_length: usize) -> Option<String> {
    let start_byte = content[..concept_match.start]
        .rfind("\n\n")
        .map(|index| index + 2)
        .unwrap_or(0);
    let end_byte = content[concept_match.end..]
        .find("\n\n")
        .map(|index| concept_match.end + index)
        .unwrap_or(content.len());

    let start_char = byte_to_char_index(content, start_byte);
    let end_char = byte_to_char_index(content, end_byte);
    ((end_char - start_char) <= max_length)
        .then(|| render_window(content, start_char, end_char, max_length))
}

fn sentence_preview(content: &str, concept_match: MatchSpan, max_length: usize) -> Option<String> {
    let match_start_char = byte_to_char_index(content, concept_match.start);
    let match_end_char = byte_to_char_index(content, concept_match.end);
    let start_char = find_last_sentence_boundary(&content[..concept_match.start])
        .map(|index| skip_leading_whitespace_chars(content, index + 1, match_start_char))
        .unwrap_or(0);
    let end_char = find_next_sentence_boundary(&content[concept_match.end..])
        .map(|index| match_end_char + index + 1)
        .unwrap_or_else(|| content.chars().count());

    ((end_char - start_char) <= max_length)
        .then(|| render_window(content, start_char, end_char, max_length))
}

fn render_window(content: &str, start_char: usize, end_char: usize, max_length: usize) -> String {
    let total_chars = content.chars().count();
    let start_char = start_char.min(total_chars);
    let end_char = end_char.min(total_chars).max(start_char);
    let excerpt = if end_char - start_char > max_length {
        let mut adjusted_end = (start_char + max_length).min(total_chars);
        if adjusted_end < end_char {
            adjusted_end = adjusted_end.max(start_char);
        }
        slice_chars(content, start_char, adjusted_end).trim()
    } else {
        slice_chars(content, start_char, end_char).trim()
    };

    let clipped_left = start_char > 0;
    let clipped_right = end_char < total_chars;
    let mut preview = String::new();
    if clipped_left {
        preview.push_str("...");
    }
    preview.push_str(excerpt);
    if clipped_right {
        preview.push_str("...");
    }
    preview
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn slice_chars(text: &str, start_char: usize, end_char: usize) -> &str {
    let start_byte = char_to_byte_index(text, start_char);
    let end_byte = char_to_byte_index(text, end_char);
    &text[start_byte..end_byte]
}

fn skip_leading_whitespace_chars(text: &str, start_char: usize, upper_bound: usize) -> usize {
    let mut index = start_char.min(upper_bound);
    let end = upper_bound.min(text.chars().count());
    while index < end {
        let next = slice_chars(text, index, index + 1);
        if !next.chars().all(char::is_whitespace) {
            break;
        }
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::extract_section_with_concept;

    #[test]
    fn centers_preview_on_mid_section_concept_mention() {
        let content = format!(
            "# Title\n\n## Notes\n\n{}\nThe focus is [[buildcontext]] because it keeps the graph traversal grounded.\n{}",
            "Intro filler. ".repeat(80),
            "Closing filler. ".repeat(80)
        );

        let preview = extract_section_with_concept(&content, "buildcontext", 160);

        assert!(preview.contains("[[buildcontext]]"));
        assert!(!preview.starts_with("## Notes"));
    }

    #[test]
    fn prefers_body_mention_over_heading_only_match() {
        let content = format!(
            "# Title\n\n## BuildContext\n\n{}\nA later [[buildcontext]] mention should drive the preview output.\n{}",
            "Heading filler. ".repeat(40),
            "Trailing filler. ".repeat(40)
        );

        let preview = extract_section_with_concept(&content, "buildcontext", 140);

        assert!(preview.contains("later [[buildcontext]] mention"));
    }

    #[test]
    fn fallback_body_preview_still_contains_plain_text_concept() {
        let content = format!(
            "{} Build context is the relevant phrase in the middle of the file. {}",
            "Lead text ".repeat(70),
            "Tail text ".repeat(70)
        );

        let preview = extract_section_with_concept(&content, "build-context", 120);

        assert!(preview.to_ascii_lowercase().contains("build context"));
    }
}
