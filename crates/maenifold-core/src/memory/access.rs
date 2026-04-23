use super::support::{
    index_file_content, learn_requested, memory_tool_help, render_filtered_wikilinks_error,
    render_learn_help, sanitize_user_input, validate_relative_folder,
};
use crate::error::{MaenifoldError, Result};
use crate::lineage::{
    gate_task_lineage_id, memory_note_lineage_id, pipeline_run_lineage_id,
    thinking_session_lineage_id,
};
use crate::markdown::{Frontmatter, read_markdown, slugify, write_markdown};
use crate::tools::ToolContext;
// T-13.5: RTM FR-8 — centralized response helpers for hints
use crate::tools::with_hint;
use crate::util::json::{OutputFormat, failure, success};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

const MAX_MEMORY_SLUG_CHARS: usize = 252;

// T-12.5: RTM FR-25.2 keeps create/read memory tool behavior stable while
// extracting the large flat [[memory.rs]] handler into focused modules.
#[allow(clippy::too_many_lines)]
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
    if let Some(folder_val) = payload.get("folder")
        && !folder_val.is_string()
        && !folder_val.is_null()
    {
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
    let folder = payload.get("folder").and_then(Value::as_str);
    let tags = payload.get("tags").and_then(Value::as_array).cloned();
    let gate_task_id = payload.get("gateTaskId").and_then(Value::as_str);
    let pipeline_session_id = payload.get("pipelineSessionId").and_then(Value::as_str);
    let thinking_session_id = payload.get("thinkingSessionId").and_then(Value::as_str);

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

    // T-89.3: Endophasia writing tap — emit concepts on successful write path only.
    for concept in &concepts {
        let _ = crate::endophasia::channel::sender().send(crate::endophasia::ConceptEvent::new(
            crate::endophasia::SignalType::Writing,
            concept.clone(),
        ));
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

    // T-14.8: RTM FR-14 - truncate slugs to a safe filename stem length so the
    // persisted `{slug}.md` component remains within common 255-byte limits
    // while preserving the full sanitized title in frontmatter metadata.
    let slug: String = slugify(&title)
        .chars()
        .take(MAX_MEMORY_SLUG_CHARS)
        .collect();
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
    let lineage_gate_task_id = gate_task_id.map(gate_task_lineage_id);
    let lineage_pipeline_run_id = pipeline_session_id.map(pipeline_run_lineage_id);
    let lineage_thinking_session_id = thinking_session_id.map(thinking_session_lineage_id);
    let lineage_memory_note_id = memory_note_lineage_id(&uri);
    if let Some(value) = lineage_gate_task_id.as_ref() {
        frontmatter.insert(
            "lineage_gate_task_id".to_string(),
            YamlValue::String(value.clone()),
        );
    }
    if let Some(value) = lineage_pipeline_run_id.as_ref() {
        frontmatter.insert(
            "lineage_pipeline_run_id".to_string(),
            YamlValue::String(value.clone()),
        );
    }
    if let Some(value) = lineage_thinking_session_id.as_ref() {
        frontmatter.insert(
            "lineage_thinking_session_id".to_string(),
            YamlValue::String(value.clone()),
        );
    }
    frontmatter.insert(
        "lineage_memory_note_id".to_string(),
        YamlValue::String(lineage_memory_note_id.clone()),
    );

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
    index_file_content(
        &uri,
        &title,
        &full_content,
        &checksum,
        lineage_gate_task_id.as_deref(),
        lineage_pipeline_run_id.as_deref(),
        lineage_thinking_session_id.as_deref(),
        Some(lineage_memory_note_id.as_str()),
        context,
    )?;

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
