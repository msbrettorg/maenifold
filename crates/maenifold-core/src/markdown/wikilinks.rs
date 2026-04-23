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
