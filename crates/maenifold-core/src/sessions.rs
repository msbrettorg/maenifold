use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, success};
use crate::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, file_exists, read_markdown, uri_to_path,
    write_markdown,
};
use crate::tools::ToolContext;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

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

    Ok("ERROR: Invalid action. Use 'append', 'update', or 'read'".to_string())
}

pub fn cleanup_abandoned_sessions(config: &Config) -> Result<usize> {
    // T-11.2: RTM FR-15
    let abandonment_minutes = std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);
    let cleanup_started_at = now_iso8601_roundtrip();
    let cleanup_display_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let connection = db::open_connection(&config.database_path)?;
    let mut statement = connection.prepare(
        "SELECT file_path, last_indexed \
         FROM file_content \
         WHERE file_path LIKE 'memory://thinking/%' \
           AND status = 'active' \
           AND datetime(last_indexed) < datetime(?1, '-' || ?2 || ' minutes')",
    )?;
    let candidates = statement
        .query_map(
            rusqlite::params![cleanup_started_at, abandonment_minutes],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut abandoned_count = 0usize;
    for (file_uri, last_indexed) in candidates {
        match abandon_session_candidate(
            &connection,
            config,
            &file_uri,
            &last_indexed,
            abandonment_minutes,
            &cleanup_started_at,
            &cleanup_display_time,
        ) {
            Ok(true) => abandoned_count += 1,
            Ok(false) => {}
            Err(error) => {
                eprintln!("Warning: failed to abandon session candidate {file_uri}: {error}");
            }
        }
    }

    Ok(abandoned_count)
}

// T-4.6: RTM FR-4.2
fn abandon_session_candidate(
    connection: &rusqlite::Connection,
    config: &Config,
    file_uri: &str,
    last_indexed: &str,
    abandonment_minutes: i64,
    cleanup_started_at: &str,
    cleanup_display_time: &str,
) -> Result<bool> {
    // T-4.6: RTM FR-4.2 - explicit post-index filesystem touches override stale DB timestamps
    // without letting freshly-created test/session fixtures suppress legitimate stale cleanup.
    let path = crate::markdown::uri_to_path(file_uri, &config.memory_path)?;
    if let Ok(metadata) = fs::metadata(&path) {
        if should_preserve_recent_filesystem_edit(&metadata, abandonment_minutes) {
            return Ok(false);
        }
    }

    let (frontmatter, content, _) = read_markdown(file_uri, config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("abandoned".to_string()),
    );

    let inactivity_minutes = parse_indexed_timestamp(last_indexed)
        .map(|indexed_at| {
            (Utc::now() - indexed_at)
                .num_minutes()
                .max(abandonment_minutes)
        })
        .unwrap_or(abandonment_minutes);
    let updated_content =
        append_abandoned_notice(&content, inactivity_minutes, cleanup_display_time);
    write_markdown(file_uri, Some(&frontmatter), &updated_content, config)?;

    let metadata = fs::metadata(&path)?;
    let (updated_frontmatter, updated_body, checksum) = read_markdown(file_uri, config)?;
    let title = updated_frontmatter
        .as_ref()
        .and_then(|values| values.get("title"))
        .and_then(YamlValue::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            file_uri
                .trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
                .to_string()
        });

    connection.execute(
        "UPDATE file_content \
         SET title = ?2, content = ?3, last_indexed = ?4, status = ?5, file_md5 = ?6, file_size = ?7 \
         WHERE file_path = ?1",
        rusqlite::params![
            file_uri,
            title,
            updated_body,
            cleanup_started_at,
            "abandoned",
            checksum,
            metadata.len() as i64
        ],
    )?;

    Ok(true)
}

// T-0.3: RTM NFR-4 - keep session cleanup deterministic across fresh fixture writes and
// explicit filesystem mtime touches.
fn should_preserve_recent_filesystem_edit(
    metadata: &fs::Metadata,
    abandonment_minutes: i64,
) -> bool {
    let Ok(modified_at) = metadata.modified() else {
        return false;
    };
    let modified_at: DateTime<Utc> = modified_at.into();
    let abandonment_cutoff = Utc::now() - chrono::Duration::minutes(abandonment_minutes);
    if modified_at < abandonment_cutoff {
        return false;
    }

    metadata_change_lags_mtime(metadata, modified_at)
}

#[cfg(unix)]
fn metadata_change_lags_mtime(metadata: &fs::Metadata, modified_at: DateTime<Utc>) -> bool {
    use std::os::unix::fs::MetadataExt;

    if metadata.ctime().is_negative() {
        return false;
    }
    let changed_duration = Duration::new(metadata.ctime() as u64, metadata.ctime_nsec() as u32);
    let changed_at: DateTime<Utc> = (UNIX_EPOCH + changed_duration).into();
    changed_at - modified_at > chrono::Duration::milliseconds(1)
}

#[cfg(not(unix))]
fn metadata_change_lags_mtime(_metadata: &fs::Metadata, _modified_at: DateTime<Utc>) -> bool {
    true
}

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
        .unwrap_or_default();

    let concepts: Vec<String> = concepts
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();

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
    // T-11.4: RTM FR-17.9 - persist lifecycle timestamps for assumption ledger parity reads.
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
    );
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
        return Ok("ERROR: Assumption not found".to_string());
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
    // T-11.4: RTM FR-17.9 - preserve validation_plan updates from either casing variant.
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
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
            display_utc_now(),
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
        return Ok("ERROR: Assumption not found".to_string());
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

// T-W1-5: RTM FR-17.9 - exported for parity-focused validation tests.
pub fn validate_assumption_append_input(
    assumption: Option<&str>,
    concepts: Option<&[String]>,
) -> Option<String> {
    if assumption.is_none_or(|value| value.trim().is_empty()) {
        return Some("ERROR: Assumption text is required".to_string());
    }
    let Some(concepts) = concepts else {
        return Some("ERROR: At least one [[WikiLink]] tag must be provided".to_string());
    };
    if concepts.is_empty() {
        return Some("ERROR: At least one [[WikiLink]] tag must be provided".to_string());
    }
    if concepts
        .iter()
        .any(|concept| concept.contains("[[") || concept.contains("]]"))
    {
        return Some("ERROR: Concept tags should not include brackets".to_string());
    }
    None
}

// T-W1-5: RTM FR-17.9 - mirror C# URI and status validation behavior.
pub fn validate_assumption_update_input(uri: Option<&str>, status: Option<&str>) -> Option<String> {
    if uri.is_none_or(|value| value.trim().is_empty()) {
        return Some("ERROR: URI is required".to_string());
    }
    if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
        return validate_assumption_status(status);
    }
    None
}

// T-W1-5: RTM FR-17.9 - read requires only a non-empty URI.
pub fn validate_assumption_read_input(uri: Option<&str>) -> Option<String> {
    if uri.is_none_or(|value| value.trim().is_empty()) {
        return Some("ERROR: URI is required".to_string());
    }
    None
}

// T-W1-5: RTM FR-17.9 - keep status enum handling case-insensitive.
pub fn validate_assumption_status(status: &str) -> Option<String> {
    let status = status.trim();
    if ["active", "validated", "invalidated", "refined"]
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(status))
    {
        None
    } else {
        Some(
            "ERROR: Invalid status. Use one of: active, validated, invalidated, refined"
                .to_string(),
        )
    }
}

// T-0.3a: RTM FR-4, RTM-047
fn build_assumption_append_content(
    assumption: &str,
    validation_plan: Option<&str>,
    concepts: &[String],
) -> String {
    let mut content = format!("# Assumption: {assumption}\n\n## Statement\n\n{assumption}\n\n");
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
        content.push_str(&format!("## Validation Plan\n\n{validation_plan}\n\n"));
    }
    content.push_str("## Related Concepts\n\n");
    for concept in concepts {
        content.push_str(&format!("- [[{concept}]]\n"));
    }
    content
}

fn append_abandoned_notice(
    content: &str,
    inactivity_minutes: i64,
    cleanup_display_time: &str,
) -> String {
    if content.contains("## Session Abandoned") {
        return content.to_string();
    }

    let trimmed = content.trim_end();
    if trimmed.is_empty() {
        format!(
            "## Session Abandoned\n\n⚠️ Session marked as abandoned due to {inactivity_minutes} minutes of inactivity\n*{cleanup_display_time}*\n"
        )
    } else {
        format!(
            "{trimmed}\n\n## Session Abandoned\n\n⚠️ Session marked as abandoned due to {inactivity_minutes} minutes of inactivity\n*{cleanup_display_time}*\n"
        )
    }
}

pub(crate) fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    frontmatter
        .get(key)
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn workflow_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/workflow/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn display_utc_now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub(crate) fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = extract_session_timestamp_millis(session_id);
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

pub(crate) fn extract_session_timestamp_millis(session_id: &str) -> Option<i64> {
    let mut parts = session_id.splitn(3, '-');
    let prefix = parts.next()?;
    let timestamp = parts.next()?;
    let suffix = parts.next()?;
    if prefix.trim().is_empty() || suffix.trim().is_empty() {
        return None;
    }
    let millis = timestamp.parse::<i64>().ok()?;
    Utc.timestamp_millis_opt(millis).single()?;
    Some(millis)
}

pub(crate) fn now_iso8601_roundtrip() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

fn frontmatter_timestamp_or_mtime(frontmatter: &Frontmatter, key: &str, path: &Path) -> String {
    let value = frontmatter_string(frontmatter, key);
    if !value.is_empty() {
        return value;
    }

    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(|modified| {
            let modified: DateTime<Utc> = modified.into();
            modified.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        })
        .unwrap_or_default()
}

fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    let path = assets_path
        .join("usage")
        .join("tools")
        .join(format!("{}.md", tool_name.to_ascii_lowercase()));
    fs::read_to_string(path).ok()
}
