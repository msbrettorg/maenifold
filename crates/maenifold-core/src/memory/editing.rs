use super::editing_helpers::{
    contains_malformed_wikilinks, count_non_overlapping_occurrences, replace_section,
};
use super::support::{
    learn_requested, memory_tool_help, render_filtered_wikilinks_error, render_learn_help,
};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{file_exists, find_file_as_uri, read_markdown, update_markdown};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure};
use serde_json::Value;
use serde_yml::Value as YamlValue;
use std::collections::BTreeMap;

// T-12.5: RTM FR-25.2 isolates edit/replace-section behavior from the
// public [[memory]] facade without changing validation or output rules.
#[allow(clippy::too_many_lines)]
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

    // T-16.3: RTM FR-25.2 — resolve identifier to URI, then surface not-found as an MCP error.
    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(found) => found,
            None => {
                return Err(MaenifoldError::Message(format!(
                    "Memory file not found: {identifier}"
                )));
            }
        }
    };

    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Memory file not found: {identifier}"
        )));
    }

    let (mut frontmatter, existing_content, actual_checksum) =
        read_markdown(&uri, &context.config)?;

    // PARITY-VAL005-ORDER: RTM NFR-1 — checksum validation BEFORE WikiLink checks,
    // matching C# order.  Error includes both expected and actual checksums.
    if let Some(expected) = checksum
        && actual_checksum != expected
    {
        return Ok(format!(
            "ERROR: Checksum mismatch. Expected: {expected}, Actual: {actual_checksum}"
        ));
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
            replace_section(&existing_content, section_name, content, &uri)?
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
