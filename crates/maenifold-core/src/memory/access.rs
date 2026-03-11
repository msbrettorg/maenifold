use super::support::{
    index_file_content, learn_requested, memory_tool_help, render_filtered_wikilinks_error,
    render_learn_help, sanitize_user_input, update_last_accessed, validate_relative_folder,
    yaml_value_to_string,
};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    Frontmatter, file_exists, find_file_as_uri, read_markdown, slugify, write_markdown,
};
use crate::tools::ToolContext;
// T-13.5: RTM FR-8 — centralized response helpers for hints
use crate::tools::with_hint;
use crate::util::json::{OutputFormat, failure, success};
use chrono::{DateTime, Local, Utc};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

// T-12.5: RTM FR-25.2 keeps create/read memory tool behavior stable while
// extracting the large flat [[memory.rs]] handler into focused modules.
pub fn write_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits WriteMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("WriteMemory")));
    }

    // PARITY-VAL002: RTM NFR-1 — reject empty/whitespace title to match C# validation.
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("title"))?;
    if title.trim().is_empty() {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Required property 'title' cannot be empty or whitespace.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Required property 'title' cannot be empty or whitespace.".to_string()
            }
        });
    }
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("content"))?;
    // PARITY-VAL003: RTM NFR-1 — reject non-string folder type to match C# type checking.
    if let Some(folder_val) = payload.get("folder") {
        if !folder_val.is_string() && !folder_val.is_null() {
            let actual_type = match folder_val {
                Value::Number(_) => "Number",
                Value::Bool(_) => "Boolean",
                Value::Array(_) => "Array",
                Value::Object(_) => "Object",
                _ => "Unknown",
            };
            let msg = format!(
                "The requested operation requires an element of type 'String', but the target element has type '{actual_type}'."
            );
            return Ok(match context.output_format {
                OutputFormat::Json => failure("TYPE_ERROR", &msg),
                OutputFormat::Markdown => format!("ERROR: {msg}"),
            });
        }
    }
    let folder = payload.get("folder").and_then(Value::as_str);
    let tags = payload.get("tags").and_then(Value::as_array).cloned();

    let title = sanitize_user_input(title);
    let concepts = crate::markdown::extract_wikilinks(content);
    if concepts.is_empty() {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.
Example: 'Learning about [[Machine Learning]] and [[Data Science]]'
This ensures your note is connected to the knowledge graph and not orphaned.".to_string(),
        });
    }

    // T-0.3c: RTM NFR-4 / RTM-047
    if let Some(folder) = folder
        && let Some(error) = validate_relative_folder(folder)
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure("PATH_VALIDATION_ERROR", error),
            OutputFormat::Markdown => format!("ERROR: Invalid folder path - {error}"),
        });
    }

    // T-14.8: RTM FR-14 - truncate overlong filesystem slugs to preserve
    // full sanitized titles in frontmatter while matching C# write behavior.
    let slug: String = slugify(&title).chars().take(255).collect();
    let uri = match folder {
        Some(folder) if !folder.is_empty() => format!("memory://{folder}/{slug}"),
        _ => format!("memory://{slug}"),
    };

    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert("title".to_string(), YamlValue::String(title.clone()));
    frontmatter.insert("permalink".to_string(), YamlValue::String(slug));
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    if let Some(tags) = tags {
        let tags = tags
            .into_iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(|text| YamlValue::String(text.to_string()))
            })
            .collect();
        frontmatter.insert("tags".to_string(), YamlValue::Sequence(tags));
    }

    let full_content = if content.trim_start().starts_with("# ") {
        content.to_string()
    } else {
        format!("# {title}\n\n{content}")
    };

    // T-2.4: RTM FR-11.2 / NFR-11.2.1 blocks writes before mutating memory files.
    if let Err(blocked) = crate::markdown::WikiLinkFilter::new(&context.config.memory_path)
        .try_check(&full_content)?
    {
        return Ok(render_filtered_wikilinks_error(
            context.output_format,
            &blocked,
        ));
    }

    write_markdown(&uri, Some(&frontmatter), &full_content, &context.config)?;
    let (_, _, checksum) = read_markdown(&uri, &context.config)?;
    index_file_content(&uri, &title, &full_content, &checksum, context)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "uri": uri,
            "checksum": checksum,
            "title": title,
            "folder": folder,
        })),
        // T-13.5: RTM FR-8 — match C# ToolResponse.WithHint("Created memory FILE:...", "WriteMemory", "Use ReadMemory with this URI to access content")
        OutputFormat::Markdown => with_hint(
            &format!("Created memory FILE: {uri}\nChecksum: {checksum}"),
            "WriteMemory",
            "Use ReadMemory with this URI to access content",
        ),
    })
}

pub fn read_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits ReadMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("ReadMemory")));
    }

    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let include_checksum = payload
        .get("includeChecksum")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    // T-16.3: RTM FR-25.2 — resolve identifier to URI, then check existence.
    // C# returns a successful "ERROR: ..." response for not-found; Rust must match.
    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(found) => found,
            None => {
                return Ok(match context.output_format {
                    OutputFormat::Json => {
                        failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
                    }
                    OutputFormat::Markdown => {
                        format!("ERROR: Memory file not found: {identifier}")
                    }
                });
            }
        }
    };

    if !file_exists(&uri, &context.config)? {
        return Ok(match context.output_format {
            OutputFormat::Json => {
                failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
            }
            OutputFormat::Markdown => format!("ERROR: Memory file not found: {identifier}"),
        });
    }

    let (frontmatter, content, checksum) = read_markdown(&uri, &context.config)?;
    update_last_accessed(&uri, context).ok();

    let title = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("title"))
        .and_then(yaml_value_to_string)
        .unwrap_or_else(|| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
                .to_string()
        });
    let location = uri.trim_start_matches("memory://").to_string();
    let created = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("created"))
        .and_then(yaml_value_to_string);
    let modified = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("modified"))
        .and_then(yaml_value_to_string);

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "uri": uri,
            "title": title,
            "location": location,
            "created": created,
            "modified": modified,
            "checksum": if include_checksum { Value::String(checksum) } else { Value::Null },
            "content": content,
        })),
        OutputFormat::Markdown => {
            // PARITY-R02-NA: show "N/A" when dates are missing (C# parity)
            // PARITY-R01-TS: format timestamps as local time, space-separated, second precision
            let created_display = created.as_deref().map(format_timestamp_local).unwrap_or_else(|| "N/A".to_string());
            let modified_display = modified.as_deref().map(format_timestamp_local).unwrap_or_else(|| "N/A".to_string());
            let mut output = format!(
                "# {title}\nURI: {uri}\nLocation: {location}\nCreated: {created_display}\nModified: {modified_display}\n",
            );
            if include_checksum {
                output.push_str(&format!("Checksum: {checksum}\n"));
            }
            output.push_str("\n---\n\n");
            output.push_str(&content);
            output
        }
    })
}

// PARITY-R01-TS: RTM NFR-1 — Convert ISO 8601 timestamps to local time format
// matching C# output: "2026-02-28 12:30:22 -08:00" (local time, space-separated,
// second precision). Falls back to raw string if parsing fails.
fn format_timestamp_local(raw: &str) -> String {
    if let Ok(utc) = raw.parse::<DateTime<Utc>>() {
        let local = utc.with_timezone(&Local);
        return local.format("%Y-%m-%d %H:%M:%S %:z").to_string();
    }
    // Try parsing with chrono's flexible parser for offset timestamps
    if let Ok(fixed) = DateTime::parse_from_rfc3339(raw) {
        let local = fixed.with_timezone(&Local);
        return local.format("%Y-%m-%d %H:%M:%S %:z").to_string();
    }
    raw.to_string()
}
