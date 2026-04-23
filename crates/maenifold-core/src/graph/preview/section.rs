// T-01.16: top-level preview construction extracted from graph/preview.rs.
// No behavioural changes.

use crate::markdown::normalize_concept;
use crate::repair::split_frontmatter_for_repair;

use super::matcher::{collect_sections, find_concept_match, ConceptMatcher, MatchSpan};
use super::text_utils::{
    byte_to_char_index, find_last_sentence_boundary, find_next_sentence_boundary, slice_chars,
    skip_leading_whitespace_chars, truncate_chars,
};

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
