use crate::util::paths::slugify_title;
use pulldown_cmark::{Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;

pub fn slugify(text: &str) -> String {
    slugify_title(text)
}

// T-2.1: RTM FR-4 preserves H2 extraction behavior for RecentActivity after the split.
pub fn extract_h2_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut heading_text = String::new();
    let mut current_body_start: Option<usize> = None;
    let mut in_h2 = false;

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                if let Some(previous_heading) = current_heading.take() {
                    let body_start = current_body_start.unwrap_or(range.start);
                    sections.push((
                        previous_heading,
                        content[body_start..range.start].trim().to_string(),
                    ));
                }
                heading_text.clear();
                in_h2 = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                current_heading = Some(heading_text.trim().to_string());
                current_body_start = Some(range.end);
                heading_text.clear();
                in_h2 = false;
            }
            Event::Text(text) | Event::Code(text) if in_h2 => heading_text.push_str(&text),
            Event::SoftBreak | Event::HardBreak if in_h2 => heading_text.push(' '),
            _ => {}
        }
    }

    if let Some(current_heading) = current_heading.take() {
        let body_start = current_body_start.unwrap_or(content.len());
        sections.push((current_heading, content[body_start..].trim().to_string()));
    }

    sections
}

#[allow(clippy::too_many_lines)]
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut visible = String::with_capacity(content.len());
    let mut code_block_depth = 0usize;
    // PARITY-INT002: RTM NFR-1 — track WikiLink spans so we can reconstruct
    // brackets in visible text, preventing phantom WikiLinks from forming when
    // pulldown_cmark decomposes nested brackets into separate text events.
    let mut in_wikilink = false;
    let bytes = content.as_bytes();

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => {
                in_wikilink = true;
                // PARITY-INT002: reconstruct opening brackets so regex_only
                // fallback sees the original structure and rejects nested brackets.
                if code_block_depth == 0 {
                    visible.push_str("[[");
                }
                let has_clean_open = range.start == 0 || bytes[range.start - 1] != b'[';
                let has_clean_close = range.end < bytes.len() && bytes[range.end] == b']';
                let has_extra_close = range.end + 1 < bytes.len() && bytes[range.end + 1] == b']';

                // PARITY-INT002: RTM NFR-1 — reject WikiLinks whose concept name
                // contains brackets, matching C# regex `[^\[\]]+` character class.
                let dest = dest_url.as_ref();
                if code_block_depth == 0
                    && has_clean_open
                    && has_clean_close
                    && !has_extra_close
                    && !dest.contains('[')
                    && !dest.contains(']')
                {
                    let link = normalize_concept(dest);
                    if !link.is_empty() && seen.insert(link.clone()) {
                        links.push(link);
                    }
                }
            }
            Event::End(TagEnd::Link) => {
                // PARITY-INT002: reconstruct closing brackets for WikiLink spans.
                if in_wikilink {
                    if code_block_depth == 0 {
                        visible.push_str("]]");
                    }
                    in_wikilink = false;
                }
            }
            Event::Code(_) => {}
            Event::Text(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                if code_block_depth == 0 {
                    visible.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if code_block_depth == 0 {
                    visible.push('\n');
                }
            }
            _ => {}
        }
    }

    for link in extract_wikilinks_regex_only(&visible) {
        if seen.insert(link.clone()) {
            links.push(link);
        }
    }

    links
}

pub fn extract_wikilinks_with_sections(content: &str) -> Vec<(String, usize)> {
    let mut sections = Vec::new();
    let mut current_section = String::new();
    let mut section_index = 0usize;

    for line in content.lines() {
        if line.starts_with("## ") {
            sections.push((section_index, std::mem::take(&mut current_section)));
            section_index += 1;
        }

        current_section.push_str(line);
        current_section.push('\n');
    }
    sections.push((section_index, current_section));

    sections
        .into_iter()
        .flat_map(|(section_index, section_content)| {
            extract_wikilinks(&section_content)
                .into_iter()
                .map(move |concept| (concept, section_index))
        })
        .collect()
}

fn extract_wikilinks_regex_only(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut index = 0;

    while index + 3 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        if index > 0 && bytes[index - 1] == b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        let mut closing = None;
        let mut invalid = false;

        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    closing = Some(cursor);
                    break;
                }
                (b'[', _) | (b']', _) => {
                    invalid = true;
                    break;
                }
                _ => cursor += 1,
            }
        }

        let Some(end) = closing else {
            index += 2;
            continue;
        };

        if invalid || (end + 2 < bytes.len() && bytes[end + 2] == b']') {
            index = end + 2;
            continue;
        }

        let link = normalize_concept(&content[index + 2..end]);
        if !link.is_empty() && seen.insert(link.clone()) {
            links.push(link);
        }
        index = end + 2;
    }

    links
}

/// Strip WikiLink bracket syntax from text for embedding.
/// Preserves the visible text, only removes `[[` and `]]` delimiters.
/// For pipe-aliased links like `[[target|display]]`, keeps the display text.
pub fn strip_wikilink_brackets(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut stripped = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '[' && chars.get(index + 1) == Some(&'[') {
            let mut cursor = index + 2;
            let mut closing = None;
            let mut invalid = false;

            while cursor + 1 < chars.len() {
                match (chars[cursor], chars[cursor + 1]) {
                    ('[', '[') => {
                        invalid = true;
                        break;
                    }
                    (']', ']') => {
                        closing = Some(cursor);
                        break;
                    }
                    ('[', _) | (']', _) => {
                        invalid = true;
                        break;
                    }
                    _ => cursor += 1,
                }
            }

            if invalid {
                if let Some(end) = find_next_closing_wikilink(&chars, cursor + 1) {
                    stripped.extend(chars[index..end].iter());
                    index = end;
                    continue;
                }

                stripped.extend(chars[index..].iter());
                break;
            }

            if let Some(end) = closing {
                let inner = chars[index + 2..end].iter().collect::<String>();
                let visible = inner
                    .split_once('|')
                    .map_or(inner.as_str(), |(_, display)| display)
                    .trim();
                append_with_boundary_spacing(&mut stripped, visible);
                if needs_boundary_space(stripped.chars().last(), chars.get(end + 2).copied()) {
                    stripped.push(' ');
                }
                index = end + 2;
                continue;
            }

            stripped.extend(chars[index..].iter());
            break;
        }

        stripped.push(chars[index]);
        index += 1;
    }

    stripped
}

fn append_with_boundary_spacing(stripped: &mut String, visible: &str) {
    if visible.is_empty() {
        return;
    }

    if needs_boundary_space(stripped.chars().last(), visible.chars().next()) {
        stripped.push(' ');
    }
    stripped.push_str(visible);
}

fn needs_boundary_space(left: Option<char>, right: Option<char>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if is_word_character(left) && is_word_character(right))
}

fn is_word_character(ch: char) -> bool {
    ch.is_alphanumeric()
}

fn find_next_closing_wikilink(chars: &[char], start: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor + 1 < chars.len() {
        if chars[cursor] == ']' && chars[cursor + 1] == ']' {
            return Some(cursor + 2);
        }
        cursor += 1;
    }
    None
}

// T-0.3: RTM NFR-4 / RTM-048 keeps normalized concept matching available across modules.
pub fn normalize_concept(concept: &str) -> String {
    let mut normalized = String::with_capacity(concept.len());
    let mut previous_was_hyphen = false;

    for ch in concept.trim().chars().flat_map(|c| c.to_lowercase()) {
        let mapped = match ch {
            ' ' | '_' | '/' => '-',
            other => other,
        };

        if mapped == '-' {
            if !previous_was_hyphen {
                normalized.push('-');
                previous_was_hyphen = true;
            }
        } else {
            normalized.push(mapped);
            previous_was_hyphen = false;
        }
    }

    normalized.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_wikilink_brackets;

    #[test]
    fn strips_basic_wikilinks() {
        assert_eq!(
            strip_wikilink_brackets("See [[concept]] today"),
            "See concept today"
        );
    }

    #[test]
    fn keeps_alias_display_text() {
        assert_eq!(
            strip_wikilink_brackets("Open [[target|display text]] now"),
            "Open display text now"
        );
    }

    #[test]
    fn inserts_spacing_to_avoid_token_merging() {
        assert_eq!(
            strip_wikilink_brackets("foo[[bar]]baz and [[zip]]zap"),
            "foo bar baz and zip zap"
        );
    }

    #[test]
    fn separates_adjacent_links_without_padding_punctuation() {
        assert_eq!(strip_wikilink_brackets("[[a]][[b]]"), "a b");
        assert_eq!(strip_wikilink_brackets("([[a]])"), "(a)");
    }

    #[test]
    fn leaves_malformed_nested_links_unchanged() {
        assert_eq!(strip_wikilink_brackets("[[foo[[bar]]"), "[[foo[[bar]]");
        assert_eq!(strip_wikilink_brackets("[[foo[[bar]]]]"), "[[foo[[bar]]]]");
    }
}
