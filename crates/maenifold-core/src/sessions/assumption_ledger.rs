use super::support::{
    frontmatter_string, frontmatter_timestamp_or_mtime, load_help_doc, now_iso8601_roundtrip,
};
use super::validation::{
    build_assumption_append_content, validate_assumption_append_input,
    validate_assumption_read_input, validate_assumption_update_input,
};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, file_exists, read_markdown, uri_to_path, write_markdown};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::{Datelike, Utc};
use serde_json::Value;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

pub fn assumption_ledger(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "AssumptionLedger")
            .unwrap_or_else(|| "ERROR: Help file not found for AssumptionLedger".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let action = payload
        .get("action")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("action"))?;

    if action.eq_ignore_ascii_case("append") {
        return append_assumption(context, payload);
    }
    if action.eq_ignore_ascii_case("update") {
        return update_assumption(context, payload);
    }
    if action.eq_ignore_ascii_case("read") {
        return read_assumption(context, payload);
    }

    Err(MaenifoldError::Message(
        "Invalid action. Use 'append', 'update', or 'read'".to_string(),
    ))
}

#[allow(clippy::too_many_lines)]
fn append_assumption(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-W1-5: RTM FR-17.9 - preserve C# assumption ledger validation contracts.
    let assumption = payload.get("assumption").and_then(Value::as_str);
    let confidence = payload
        .get("confidence")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    let context_text = payload.get("context").and_then(Value::as_str);
    let validation_plan = payload
        .get("validationPlan")
        .and_then(Value::as_str)
        .or_else(|| payload.get("validation_plan").and_then(Value::as_str));
    let concepts = payload
        .get("concepts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();

    if let Some(message) = validate_assumption_append_input(assumption, Some(&concepts)) {
        return Ok(message);
    }

    let assumption = assumption.expect("validated assumption");
    let now = Utc::now();
    let entry_id = format!("assumption-{}", now.timestamp_millis());
    let uri = format!(
        "memory://assumptions/{:04}/{:02}/{}",
        now.year(),
        now.month(),
        entry_id
    );

    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "confidence".to_string(),
        YamlValue::String(confidence.to_string()),
    );
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
    );
    // T-11.4: RTM FR-17.9 - persist lifecycle timestamps for assumption ledger parity reads.
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String(now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
    );
    if let Some(context_text) = context_text.filter(|value| !value.trim().is_empty()) {
        frontmatter.insert(
            "context".to_string(),
            YamlValue::String(context_text.to_string()),
        );
    }
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
        frontmatter.insert(
            "validation_plan".to_string(),
            YamlValue::String(validation_plan.to_string()),
        );
    }

    let content = build_assumption_append_content(assumption, validation_plan, &concepts);
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)?;

    let mut output = format!(
        "✅ Assumption recorded at {uri}\n\n**Statement**: {assumption}\n**Confidence**: {confidence}\n**Status**: active"
    );
    if let Some(context_text) = context_text.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("\n**Context**: {context_text}"));
    }
    output.push_str(
        "\n\n💡 Next: Consider running Sync() to index this assumption into the knowledge graph for SearchMemories discovery.\n",
    );
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!(
            "💡 Remember to update status after completing: {validation_plan}\n"
        ));
    }

    Ok(output)
}

fn update_assumption(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-W1-5: RTM FR-17.9 - port update lifecycle semantics from the C# oracle.
    let uri = payload.get("uri").and_then(Value::as_str);
    let status = payload.get("status").and_then(Value::as_str);
    let notes = payload.get("notes").and_then(Value::as_str);
    let confidence = payload.get("confidence").and_then(Value::as_str);
    let validation_plan = payload
        .get("validationPlan")
        .and_then(Value::as_str)
        .or_else(|| payload.get("validation_plan").and_then(Value::as_str));

    if let Some(message) = validate_assumption_update_input(uri, status) {
        return Ok(message);
    }

    let uri = uri.expect("validated uri");
    if !file_exists(uri, &context.config)? {
        return Err(MaenifoldError::Message("Assumption not found".to_string()));
    }

    let (frontmatter, content, _) = read_markdown(uri, &context.config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();
    let mut updated_content = content;

    let normalized_status = status
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_ascii_lowercase());
    if let Some(status) = normalized_status.as_ref() {
        frontmatter.insert("status".to_string(), YamlValue::String(status.clone()));
    }
    if let Some(confidence) = confidence.filter(|value| !value.trim().is_empty()) {
        frontmatter.insert(
            "confidence".to_string(),
            YamlValue::String(confidence.trim().to_string()),
        );
    }
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
        // T-11.4: RTM FR-17.9 - preserve validation_plan updates from either casing variant.
        frontmatter.insert(
            "validation_plan".to_string(),
            YamlValue::String(validation_plan.trim().to_string()),
        );
    }
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );

    if let Some(notes) = notes.filter(|value| !value.trim().is_empty()) {
        if !updated_content.ends_with('\n') {
            updated_content.push('\n');
        }
        updated_content.push_str(&format!(
            "\n## Update: {}\n\n{}\n",
            super::display_utc_now(),
            notes.trim()
        ));
    }

    write_markdown(uri, Some(&frontmatter), &updated_content, &context.config)?;

    let mut output = format!("✅ Assumption updated at {uri}");
    if let Some(status) = normalized_status {
        output.push_str(&format!("\n**New Status**: {status}"));
    }
    if let Some(confidence) = confidence.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("\n**New Confidence**: {}", confidence.trim()));
    }
    Ok(output)
}

fn read_assumption(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-W1-5: RTM FR-17.9 - support direct inspection of recorded assumptions.
    let uri = payload.get("uri").and_then(Value::as_str);
    if let Some(message) = validate_assumption_read_input(uri) {
        return Ok(message);
    }

    let uri = uri.expect("validated uri");
    if !file_exists(uri, &context.config)? {
        return Err(MaenifoldError::Message("Assumption not found".to_string()));
    }

    let (frontmatter, content, _) = read_markdown(uri, &context.config)?;
    let frontmatter = frontmatter.unwrap_or_default();
    let path = uri_to_path(uri, &context.config.memory_path)?;
    let status = frontmatter_string(&frontmatter, "status");
    let confidence = frontmatter_string(&frontmatter, "confidence");
    let context_text = frontmatter_string(&frontmatter, "context");
    let created = frontmatter_timestamp_or_mtime(&frontmatter, "created", &path);
    let modified = frontmatter_timestamp_or_mtime(&frontmatter, "modified", &path);
    let validation_plan = frontmatter_string(&frontmatter, "validation_plan");

    let mut output = format!(
        "📋 **Assumption**: {uri}\n\n**Status**: {status}\n**Confidence**: {confidence}\n**Created**: {created}\n**Modified**: {modified}"
    );
    if !context_text.is_empty() {
        output.push_str(&format!("\n**Context**: {context_text}"));
    }
    if !validation_plan.is_empty() {
        output.push_str(&format!("\n**Validation Plan**: {validation_plan}"));
    }
    output.push_str("\n\n## Content\n\n");
    output.push_str(&content);
    Ok(output)
}
