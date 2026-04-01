use crate::error::{MaenifoldError, Result};

pub(super) fn replace_section(
    existing_content: &str,
    section_name: &str,
    new_content: &str,
    file_context: &str,
) -> Result<String> {
    let mut line_start = 0usize;
    let mut section_start = None;
    let mut heading_end = None;
    let mut section_end = existing_content.len();

    for line in existing_content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let heading_text = trimmed.trim_end().trim_start_matches('#').trim();
            if section_start.is_some() {
                section_end = line_start;
                break;
            }
            // T-15.3: RTM FR-15 partial heading match — C# regex uses
            // `{sectionName}[^\n]*` which matches headings that start with the
            // provided section name text.
            if heading_text.starts_with(section_name) {
                section_start = Some(line_start);
                heading_end = Some(line_start + line.len());
            }
        }
        line_start += line.len();
    }

    let section_start = section_start.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Section not found: '{section_name}'. Cannot replace a section that doesn't exist."
        ))
    })?;
    let heading_end = heading_end.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Failed to determine the heading boundary for section '{section_name}' in '{file_context}'. The markdown heading may be malformed."
        ))
    })?;

    // T-15.4: RTM FR-15 heading preservation — C# always keeps the old heading
    // via regex backreference $2.  When new content starts with a heading,
    // strip it so we don't produce a duplicate heading line.
    let replacement = if starts_with_markdown_heading(new_content) {
        let stripped = strip_leading_heading(new_content);
        format!(
            "{}{}",
            &existing_content[section_start..heading_end],
            stripped.trim_end()
        )
    } else {
        format!(
            "{}{}",
            &existing_content[section_start..heading_end],
            new_content.trim_end()
        )
    };

    Ok(format!(
        "{}{}{}",
        &existing_content[..section_start],
        replacement,
        &existing_content[section_end..]
    ))
}

pub(super) fn count_non_overlapping_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

pub(super) fn starts_with_markdown_heading(content: &str) -> bool {
    content
        .trim_start()
        .lines()
        .next()
        .map(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#') && trimmed.trim_start_matches('#').starts_with(' ')
        })
        .unwrap_or(false)
}

/// T-15.4: RTM FR-15 strips the first markdown heading line from content,
/// returning everything after the heading's trailing newline.
pub(super) fn strip_leading_heading(content: &str) -> &str {
    let trimmed = content.trim_start();
    match trimmed.find('\n') {
        Some(pos) => &trimmed[pos + 1..],
        None => "",
    }
}

pub(super) fn contains_malformed_wikilinks(content: &str) -> bool {
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index + 1 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    if cursor + 2 < bytes.len() && bytes[cursor + 2] == b']' {
                        return true;
                    }
                    index = cursor + 2;
                    break;
                }
                (b'[', _) | (b']', _) => return true,
                _ => cursor += 1,
            }
        }

        if cursor + 1 >= bytes.len() {
            return true;
        }
    }

    false
}
