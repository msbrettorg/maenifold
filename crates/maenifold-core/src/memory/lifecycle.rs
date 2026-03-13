use super::support::{destination_to_uri, learn_requested, memory_tool_help, render_learn_help};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{delete_file, file_exists, find_file_as_uri, validate_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure, success};
use serde_json::{Value, json};
use std::fs;

// T-12.5: RTM FR-25.2 isolates delete/move operations while keeping the
// public [[memory]] entrypoints and messages unchanged.
pub fn delete_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits DeleteMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("DeleteMemory")));
    }

    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let confirm = payload
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !confirm {
        // T-1.6: RTM NFR-1 - confirmation failures stay in the JSON envelope.
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "CONFIRMATION_REQUIRED",
                "Must set confirm=true to delete a memory file",
            ),
            OutputFormat::Markdown => {
                "ERROR: Must set confirm=true to delete a memory file".to_string()
            }
        });
    }

    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(uri) => uri,
            None => {
                return Ok(match context.output_format {
                    OutputFormat::Json => {
                        failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
                    }
                    OutputFormat::Markdown => format!("ERROR: Memory file not found: {identifier}"),
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

    delete_file(&uri, &context.config)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "uri": uri, "deleted": true })),
        OutputFormat::Markdown => format!("Deleted memory FILE: {uri}"),
    })
}

pub fn move_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    if learn_requested(payload) {
        // T-11.5: RTM FR-25.2 / NFR-1 short-circuits MoveMemory I/O with inline learn content.
        return Ok(render_learn_help(context, memory_tool_help("MoveMemory")));
    }

    let source = payload
        .get("source")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("source"))?;
    let destination = payload
        .get("destination")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("destination"))?;

    // T-15.5: RTM FR-25.2 — reject empty/whitespace-only destination early
    // to prevent silent URI construction bugs (matches C# MoveMemory validation).
    if destination.trim().is_empty() {
        return Ok("ERROR: Destination parameter is required and cannot be empty.".to_string());
    }

    // T-16.3: RTM FR-25.2 — resolve identifier to URI, then check existence.
    // C# returns a successful "ERROR: ..." response for source-not-found; Rust must match.
    let source_uri = if source.starts_with("memory://") {
        source.to_string()
    } else {
        match find_file_as_uri(source, &context.config) {
            Some(found) => found,
            None => {
                return Ok(format!("ERROR: Source memory file not found: {source}"));
            }
        }
    };

    if !file_exists(&source_uri, &context.config)? {
        return Ok(format!("ERROR: Source memory file not found: {source}"));
    }

    let source_path = crate::markdown::uri_to_path(&source_uri, &context.config.memory_path)?;

    let destination_uri = destination_to_uri(destination);
    validate_uri(&destination_uri, "destination")?;
    let destination_path =
        crate::markdown::uri_to_path(&destination_uri, &context.config.memory_path)?;
    if destination_path.exists() {
        return Ok(format!(
            "ERROR: Destination file already exists: {destination_uri}"
        ));
    }
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(source_path, destination_path)?;

    Ok(format!(
        "Moved memory FILE: {source_uri} → {destination_uri}"
    ))
}
