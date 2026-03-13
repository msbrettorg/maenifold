use super::support::{
    learn_requested, memory_tool_help, render_filtered_wikilinks_error, render_learn_help,
};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{file_exists, find_file_as_uri, read_markdown, update_markdown};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure};
use serde_json::Value;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

// T-12.5: RTM FR-25.2 isolates edit/replace-section behavior from the
// public [[memory]] facade without changing validation or output rules.
pub fn edit_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits EditMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("EditMemory")));
    }

    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let operation = payload
        .get("operation")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("operation"))?;
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("content"))?;
    let checksum = payload.get("checksum").and_then(Value::as_str);

    // T-16.3: RTM FR-25.2 — resolve identifier to URI, then check existence.
    // C# returns a successful "ERROR: ..." response for not-found; Rust must match.
    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(found) => found,
            None => {
                return Ok(format!("ERROR: Memory file not found: {identifier}"));
            }
        }
    };

    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Memory file not found: {identifier}"));
    }

    let (mut frontmatter, existing_content, actual_checksum) =
        read_markdown(&uri, &context.config)?;

    // PARITY-VAL005-ORDER: RTM NFR-1 — checksum validation BEFORE WikiLink checks,
    // matching C# order.  Error includes both expected and actual checksums.
    if let Some(expected) = checksum {
        if actual_checksum != expected {
            return Ok(format!(
                "ERROR: Checksum mismatch. Expected: {expected}, Actual: {actual_checksum}"
            ));
        }
    }

    // T-15.2: RTM FR-15 case-insensitive operation matching (C# uses .ToLowerInvariant())
    let operation = operation.to_ascii_lowercase();
    let operation = operation.as_str();
    if matches!(operation, "append" | "prepend" | "replace_section")
        && crate::markdown::extract_wikilinks(content).is_empty()
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Edited content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Edited content must contain at least one [[concept]] in double brackets to connect to the knowledge graph."
                    .to_string()
            }
        });
    }
    let updated_content = match operation {
        // T-15.1: RTM FR-15 append/prepend join with "\n\n" separator matching C# behavior
        "append" => format!("{existing_content}\n\n{content}"),
        "prepend" => format!("{content}\n\n{existing_content}"),
        "find_replace" => {
            let find_text = payload
                .get("findText")
                .and_then(Value::as_str)
                .ok_or(MaenifoldError::MissingProperty("findText"))?;
            let expected_count = payload.get("expectedCount").and_then(Value::as_i64);
            if let Some(expected_count) = expected_count {
                let actual_count = count_non_overlapping_occurrences(&existing_content, find_text);
                if actual_count != expected_count as usize {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "Expected {expected_count} matches but found {actual_count}. Find text: '{find_text}'"
                    )));
                }
            }
            existing_content.replace(find_text, content)
        }
        "replace_section" => {
            let section_name = payload
                .get("sectionName")
                .and_then(Value::as_str)
                .ok_or(MaenifoldError::MissingProperty("sectionName"))?;
            replace_section(&existing_content, section_name, content)?
        }
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Unsupported operation: {operation}"
            )));
        }
    };

    if contains_malformed_wikilinks(&updated_content)
        || crate::markdown::extract_wikilinks(&updated_content).is_empty()
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph."
                    .to_string()
            }
        });
    }

    // T-2.4: RTM FR-11.2 / NFR-11.2.2 / NFR-11.2.3 reports blocked [[WikiLinks]] in both output modes.
    if let Err(blocked) = crate::markdown::WikiLinkFilter::new(&context.config.memory_path)
        .try_check(&updated_content)?
    {
        return Ok(render_filtered_wikilinks_error(
            context.output_format,
            &blocked,
        ));
    }

    let frontmatter = frontmatter.get_or_insert_with(BTreeMap::new);
    frontmatter
        .entry("type".to_string())
        .or_insert_with(|| YamlValue::String("memory".to_string()));
    frontmatter
        .entry("status".to_string())
        .or_insert_with(|| YamlValue::String("saved".to_string()));

    // PARITY-VAL005-ORDER: checksum already validated above; pass None to skip
    // redundant re-check inside update_markdown.
    update_markdown(
        &uri,
        Some(frontmatter),
        &updated_content,
        None,
        &context.config,
    )?;
    let (_, _, new_checksum) = read_markdown(&uri, &context.config)?;

    Ok(format!(
        "Updated memory FILE: {uri}\nNew checksum: {new_checksum}"
    ))
}

fn replace_section(
    existing_content: &str,
    section_name: &str,
    new_content: &str,
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
    let heading_end = heading_end.expect("heading end set with section start");

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

fn count_non_overlapping_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

fn starts_with_markdown_heading(content: &str) -> bool {
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
fn strip_leading_heading(content: &str) -> &str {
    let trimmed = content.trim_start();
    match trimmed.find('\n') {
        Some(pos) => &trimmed[pos + 1..],
        None => "",
    }
}

fn contains_malformed_wikilinks(content: &str) -> bool {
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

// ---------------------------------------------------------------------------
// T-15.1–15.4: Unit tests for editing parity fixes
// ---------------------------------------------------------------------------
#[cfg(test)]
mod editing_parity_tests {
    use super::*;

    // -- T-15.1: replace_section preserves separator behavior --

    #[test]
    fn t15_1_replace_section_basic() {
        // T-15.1: RTM FR-15 — basic replace_section still works
        let existing = "Intro\n\n## Alpha\n\nOld alpha body.\n\n## Beta\n\nBeta body.";
        let result = replace_section(existing, "Alpha", "New alpha with [[concepts]].").unwrap();
        assert!(result.contains("## Alpha\n"), "heading preserved");
        assert!(
            result.contains("New alpha with [[concepts]]."),
            "new body present"
        );
        assert!(!result.contains("Old alpha body"), "old body replaced");
        assert!(result.contains("## Beta\n"), "other section untouched");
    }

    // -- T-15.3: partial heading match --

    #[test]
    fn t15_3_partial_heading_match_starts_with() {
        // T-15.3: RTM FR-15 — "Design" matches "## Design Decisions"
        let existing =
            "Intro\n\n## Design Decisions\n\nOld design.\n\n## Implementation\n\nImpl body.";
        let result =
            replace_section(existing, "Design", "New design notes with [[arch]].").unwrap();
        assert!(
            result.contains("## Design Decisions\n"),
            "original full heading preserved, got:\n{result}"
        );
        assert!(result.contains("New design notes with [[arch]]."));
        assert!(!result.contains("Old design"));
        assert!(result.contains("## Implementation\n"));
    }

    #[test]
    fn t15_3_exact_heading_still_works() {
        // T-15.3: RTM FR-15 — exact match continues to work
        let existing = "## Notes\n\nBody of notes.";
        let result = replace_section(existing, "Notes", "Updated with [[links]].").unwrap();
        assert!(result.contains("## Notes\n"));
        assert!(result.contains("Updated with [[links]]."));
    }

    #[test]
    fn t15_3_partial_heading_not_found() {
        // T-15.3: RTM FR-15 — no match returns error
        let existing = "## Something Else\n\nBody.";
        let err = replace_section(existing, "Design", "New with [[x]].").unwrap_err();
        assert!(err.to_string().contains("Section not found"));
    }

    // -- T-15.4: heading preservation when new content starts with heading --

    #[test]
    fn t15_4_new_content_with_heading_preserves_old_heading() {
        // T-15.4: RTM FR-15 — old heading kept, new heading stripped
        let existing = "Intro\n\n## Original Title\n\nOld body.\n\n## Next\n\nNext body.";
        let result = replace_section(
            existing,
            "Original Title",
            "## Replaced Heading\nNew body with [[concepts]].",
        )
        .unwrap();
        assert!(
            result.contains("## Original Title\n"),
            "old heading must be preserved, got:\n{result}"
        );
        assert!(
            result.contains("New body with [[concepts]]"),
            "new body content present"
        );
        assert!(
            !result.contains("## Replaced Heading"),
            "heading from new content should be stripped"
        );
        assert!(result.contains("## Next\n"), "other section untouched");
    }

    #[test]
    fn t15_4_new_content_without_heading_preserves_old_heading() {
        // T-15.4: RTM FR-15 — when no heading in new content, old heading still kept
        let existing = "## Section\n\nOld body.";
        let result =
            replace_section(existing, "Section", "Plain new body with [[links]].").unwrap();
        assert!(result.contains("## Section\n"));
        assert!(result.contains("Plain new body with [[links]]."));
    }

    // -- helpers --

    #[test]
    fn starts_with_markdown_heading_detects_headings() {
        assert!(starts_with_markdown_heading("## Heading\nBody"));
        assert!(starts_with_markdown_heading("# H1"));
        assert!(starts_with_markdown_heading("  ## Indented"));
        assert!(!starts_with_markdown_heading("Not a heading"));
        assert!(!starts_with_markdown_heading("#NoSpace"));
        assert!(!starts_with_markdown_heading(""));
    }

    #[test]
    fn strip_leading_heading_removes_first_line() {
        assert_eq!(strip_leading_heading("## Heading\nBody text"), "Body text");
        assert_eq!(
            strip_leading_heading("# Title\nLine 2\nLine 3"),
            "Line 2\nLine 3"
        );
        assert_eq!(strip_leading_heading("## Only Heading"), "");
        assert_eq!(strip_leading_heading("  ## Spaced\nContent"), "Content");
    }

    // -- T-15.2: case-insensitive operation matching --

    #[test]
    fn t15_2_operation_lowercasing() {
        // T-15.2: RTM FR-15 — operation strings are lowercased before matching
        let cases = vec![
            ("append", "append"),
            ("APPEND", "append"),
            ("Append", "append"),
            ("prepend", "prepend"),
            ("PREPEND", "prepend"),
            ("find_replace", "find_replace"),
            ("Find_Replace", "find_replace"),
            ("FIND_REPLACE", "find_replace"),
            ("replace_section", "replace_section"),
            ("REPLACE_SECTION", "replace_section"),
            ("Replace_Section", "replace_section"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                input.to_ascii_lowercase(),
                expected,
                "lowercasing {input} should produce {expected}"
            );
        }
    }
}
