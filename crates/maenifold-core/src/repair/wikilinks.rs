use crate::error::{MaenifoldError, Result};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug)]
struct RepairEdit {
    start: usize,
    end: usize,
}

// T-3.1: RTM FR-6
pub(super) fn replace_wikilink_variants(
    content: &str,
    variants: &[String],
    canonical_concept: &str,
) -> Result<(String, Vec<String>, usize)> {
    let mut edits = Vec::<RepairEdit>::new();
    let mut variant_counts = HashMap::<String, usize>::new();

    for variant in variants {
        let pattern =
            Regex::new(&format!(r"(?i)\[\[{}\]\]", regex::escape(variant))).map_err(|error| {
                MaenifoldError::InvalidArgument(format!(
                    "Invalid WikiLink repair pattern for '{variant}': {error}"
                ))
            })?;
        for found in pattern.find_iter(content) {
            edits.push(RepairEdit {
                start: found.start(),
                end: found.end(),
            });
            *variant_counts.entry(variant.clone()).or_default() += 1;
        }
    }

    if edits.is_empty() {
        return Ok((content.to_string(), Vec::new(), 0));
    }

    edits.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });

    let mut updated_content = content.to_string();
    let replacement = format!("[[{}]]", canonical_concept);
    for edit in edits {
        updated_content.replace_range(edit.start..edit.end, &replacement);
    }

    let matched_variants = variants
        .iter()
        .filter(|variant| variant_counts.contains_key(*variant))
        .cloned()
        .collect::<Vec<_>>();
    let total_replacements = variant_counts.values().sum();

    Ok((updated_content, matched_variants, total_replacements))
}

// T-3.1: RTM FR-6
pub(super) fn create_wikilinks_for_plain_text_variants(
    content: &str,
    variants: &[String],
    canonical_concept: &str,
) -> Result<(String, Vec<String>, usize)> {
    let (_, body, body_offset) = split_frontmatter_for_repair(content);
    let existing_wikilink_ranges = find_existing_wikilink_ranges(body)?;
    let variant_patterns = variants
        .iter()
        .map(|variant| (variant.clone(), variant.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let mut edits = Vec::<RepairEdit>::new();
    let mut variant_counts = HashMap::<String, usize>::new();
    let mut paragraph_depth = 0usize;
    let mut heading_depth = 0usize;
    let mut item_depth = 0usize;
    let mut link_depth = 0usize;
    let mut code_block_depth = 0usize;

    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => paragraph_depth += 1,
                Tag::Heading { .. } => heading_depth += 1,
                Tag::Item => item_depth += 1,
                Tag::Link { .. } => link_depth += 1,
                Tag::CodeBlock(_) => code_block_depth += 1,
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => paragraph_depth = paragraph_depth.saturating_sub(1),
                TagEnd::Heading(_) => heading_depth = heading_depth.saturating_sub(1),
                TagEnd::Item => item_depth = item_depth.saturating_sub(1),
                TagEnd::Link => link_depth = link_depth.saturating_sub(1),
                TagEnd::CodeBlock => code_block_depth = code_block_depth.saturating_sub(1),
                _ => {}
            },
            Event::Text(_) => {
                if code_block_depth == 0
                    && link_depth == 0
                    && (paragraph_depth > 0 || heading_depth > 0 || item_depth > 0)
                {
                    collect_plain_text_repair_edits(
                        body,
                        range,
                        body_offset,
                        &variant_patterns,
                        &existing_wikilink_ranges,
                        &mut edits,
                        &mut variant_counts,
                    );
                }
            }
            _ => {}
        }
    }

    if edits.is_empty() {
        return Ok((content.to_string(), Vec::new(), 0));
    }

    edits.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });

    let mut updated_content = content.to_string();
    let replacement = format!("[[{canonical_concept}]]");
    for edit in edits {
        updated_content.replace_range(edit.start..edit.end, &replacement);
    }

    let matched_variants = variants
        .iter()
        .filter(|variant| variant_counts.contains_key(*variant))
        .cloned()
        .collect::<Vec<_>>();
    let total_replacements = variant_counts.values().sum();

    Ok((updated_content, matched_variants, total_replacements))
}

fn collect_plain_text_repair_edits(
    body: &str,
    range: std::ops::Range<usize>,
    body_offset: usize,
    variant_patterns: &[(String, String)],
    existing_wikilink_ranges: &[std::ops::Range<usize>],
    edits: &mut Vec<RepairEdit>,
    variant_counts: &mut HashMap<String, usize>,
) {
    let segment = &body[range.clone()];
    if segment.is_empty() {
        return;
    }

    let lower_segment = segment.to_ascii_lowercase();
    let mut candidates = Vec::<(usize, usize, &str)>::new();

    for (variant, variant_lower) in variant_patterns {
        for (local_start, _) in lower_segment.match_indices(variant_lower) {
            let local_end = local_start + variant_lower.len();
            if !is_whole_word_match(segment, local_start, local_end) {
                continue;
            }

            let absolute_start = body_offset + range.start + local_start;
            let absolute_end = body_offset + range.start + local_end;
            if overlaps_existing_wikilink(
                absolute_start - body_offset,
                absolute_end - body_offset,
                existing_wikilink_ranges,
            ) {
                continue;
            }

            candidates.push((absolute_start, absolute_end, variant));
        }
    }

    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| (right.1 - right.0).cmp(&(left.1 - left.0)))
            .then_with(|| left.2.cmp(right.2))
    });

    let mut last_end = 0usize;
    for (start, end, variant) in candidates {
        if start < last_end {
            continue;
        }
        edits.push(RepairEdit { start, end });
        *variant_counts.entry(variant.to_string()).or_default() += 1;
        last_end = end;
    }
}

// T-3.1: RTM FR-6
pub(crate) fn split_frontmatter_for_repair(content: &str) -> (&str, &str, usize) {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(index) = rest.find("\n---\n") {
            let mut body_start = 4 + index + 5;
            while let Some(next) = content.as_bytes().get(body_start) {
                if *next == b'\n' || *next == b'\r' {
                    body_start += 1;
                } else {
                    break;
                }
            }
            return (&content[..body_start], &content[body_start..], body_start);
        }
    }

    ("", content, 0)
}

fn find_existing_wikilink_ranges(content: &str) -> Result<Vec<std::ops::Range<usize>>> {
    let pattern = Regex::new(r"\[\[[^\]]+\]\]").map_err(|error| {
        MaenifoldError::InvalidArgument(format!("Invalid WikiLink detection pattern: {error}"))
    })?;

    Ok(pattern
        .find_iter(content)
        .map(|found| found.start()..found.end())
        .collect())
}

fn is_whole_word_match(segment: &str, start: usize, end: usize) -> bool {
    let bytes = segment.as_bytes();
    let left_boundary = start == 0 || !is_word_byte(bytes[start - 1]);
    let right_boundary = end == bytes.len() || !is_word_byte(bytes[end]);
    left_boundary && right_boundary
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn overlaps_existing_wikilink(
    start: usize,
    end: usize,
    existing_wikilink_ranges: &[std::ops::Range<usize>],
) -> bool {
    existing_wikilink_ranges
        .iter()
        .any(|range| start < range.end && range.start < end)
}
