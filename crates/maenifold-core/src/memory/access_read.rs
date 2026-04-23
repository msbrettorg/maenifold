//! T-01.17: ReadMemory handler split from [`super::access`] to keep the
//! parent file under the per-file LOC budget. No behavioral changes.

use super::support::{
    learn_requested, memory_tool_help, render_learn_help, update_last_accessed,
    yaml_value_to_string,
};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{file_exists, find_file_as_uri, read_markdown};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure, success};
use chrono::{DateTime, Local, Utc};
use serde_json::{Value, json};

#[allow(clippy::too_many_lines)]
pub fn read_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits ReadMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("ReadMemory")));
    }

    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let _ = crate::endophasia::channel::sender().send(crate::endophasia::ConceptEvent::new(
        crate::endophasia::SignalType::Remembering,
        identifier.to_string(),
    ));
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
            let created_display = created
                .as_deref()
                .map(format_timestamp_local)
                .unwrap_or_else(|| "N/A".to_string());
            let modified_display = modified
                .as_deref()
                .map(format_timestamp_local)
                .unwrap_or_else(|| "N/A".to_string());
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
