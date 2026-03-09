use crate::assets::AssetManager;
use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, failure, success};
use crate::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::tools::ToolContext;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn sequential_thinking(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "SequentialThinking")
            .unwrap_or_else(|| "ERROR: Help file not found for SequentialThinking".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let cancel = payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response = payload.get("response").and_then(Value::as_str);
    let thoughts = payload.get("thoughts").and_then(Value::as_str);
    let thought_number = payload
        .get("thoughtNumber")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_thoughts = payload
        .get("totalThoughts")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next_thought_needed = payload
        .get("nextThoughtNeeded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let needs_more_thoughts = payload
        .get("needsMoreThoughts")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let is_revision = payload
        .get("isRevision")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let revises_thought = payload.get("revisesThought").and_then(Value::as_u64);
    let branch_from_thought = payload.get("branchFromThought").and_then(Value::as_u64);
    let branch_id = payload.get("branchId").and_then(Value::as_str);
    let analysis_type = payload.get("analysisType").and_then(Value::as_str);
    let parent_workflow_id = payload.get("parentWorkflowId").and_then(Value::as_str);
    let conclusion = payload.get("conclusion").and_then(Value::as_str);
    let session_id_provided = payload.get("sessionId").and_then(Value::as_str).is_some();
    let session_id = payload
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(generate_sequential_session_id);

    if !cancel && thought_number > 0 && payload.get("sessionId").is_none() {
        // T-1.6: RTM NFR-1 - follow-on thoughts require an existing session.
        let message = format!("Session {session_id} missing. Start with thoughtNumber=0.");
        return Ok(match context.output_format {
            OutputFormat::Json => failure("SESSION_MISSING", message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }

    if !cancel {
        if response.is_none() {
            return Err(MaenifoldError::MissingProperty("response"));
        }
        let total_concepts = response.map(extract_wikilinks).unwrap_or_default().len()
            + thoughts.map(extract_wikilinks).unwrap_or_default().len();
        if total_concepts == 0 {
            let message =
                "Must include [[WikiLinks]]. Example: 'Analyzing [[Machine Learning]] algorithms'";
            return Ok(match context.output_format {
                OutputFormat::Json => failure("WIKILINK_REQUIRED", message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }
    }

    if branch_from_thought.is_some() && branch_id.is_none() {
        let message =
            "branchId required when branchFromThought is specified for multi-agent coordination";
        return Ok(match context.output_format {
            OutputFormat::Json => failure("BRANCH_ID_REQUIRED", message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }

    // T-W1-5: RTM FR-17.9 - parent workflow links are only valid for the first thought.
    if let Some(parent_workflow_id) = parent_workflow_id.filter(|value| !value.trim().is_empty()) {
        if thought_number > 0 {
            let message = "Parent workflow can only be set on first thought";
            return Ok(match context.output_format {
                OutputFormat::Json => failure("PARENT_WORKFLOW_FIRST_THOUGHT_ONLY", message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }

        let parent_uri = workflow_session_uri(parent_workflow_id);
        if !file_exists(&parent_uri, &context.config)? {
            let message = format!("Parent workflow '{parent_workflow_id}' not found");
            return Ok(match context.output_format {
                OutputFormat::Json => failure("PARENT_WORKFLOW_NOT_FOUND", &message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }
    }

    let (year, month, day) = date_parts_from_session_id(&session_id);
    let uri = format!("memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{session_id}");
    let session_exists = file_exists(&uri, &context.config)?;

    // T-0.2: RTM NFR-3 - first thoughts must start at 0 unless continuing an existing session.
    if session_id_provided && thought_number == 0 && !session_exists && !is_revision {
        let message = format!(
            "Session {session_id} not found. To start new session, don't provide sessionId."
        );
        return Ok(match context.output_format {
            OutputFormat::Json => failure("SESSION_NOT_FOUND", &message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }
    if thought_number == 0 && branch_id.is_none() && session_exists && !is_revision && !cancel {
        let message =
            format!("Session {session_id} exists. Use different sessionId or continue existing.");
        return Ok(match context.output_format {
            OutputFormat::Json => failure("SESSION_EXISTS", &message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }
    if thought_number > 0 && !session_exists && !is_revision && !cancel {
        let message = format!("Session {session_id} missing. Start with thoughtNumber=0.");
        return Ok(match context.output_format {
            OutputFormat::Json => failure("SESSION_MISSING", &message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }
    if is_revision && !session_exists {
        let message = format!("Cannot revise - session {session_id} doesn't exist.");
        return Ok(match context.output_format {
            OutputFormat::Json => failure("SESSION_NOT_FOUND", &message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }

    if !session_exists {
        let mut frontmatter: Frontmatter = BTreeMap::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Sequential Thinking Session {session_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(session_id.clone()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String("sequential".to_string()),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        if let Some(analysis_type) = analysis_type
            && !analysis_type.is_empty()
        {
            frontmatter.insert(
                "analysisType".to_string(),
                YamlValue::String(analysis_type.to_string()),
            );
        }
        if let Some(parent_workflow_id) = parent_workflow_id
            && !parent_workflow_id.is_empty()
        {
            frontmatter.insert(
                "parent".to_string(),
                YamlValue::String(format!("[[workflow/{parent_workflow_id}]]")),
            );
        }
        write_markdown(
            &uri,
            Some(&frontmatter),
            "# Sequential Thinking Session\n\n",
            &context.config,
        )?;

        if let Some(parent_workflow_id) =
            parent_workflow_id.filter(|value| !value.trim().is_empty())
        {
            let parent_uri = workflow_session_uri(parent_workflow_id);
            let (parent_frontmatter, parent_content, _) =
                read_markdown(&parent_uri, &context.config)?;
            let mut parent_frontmatter = parent_frontmatter.unwrap_or_default();
            let related_link = format!("[[sequential/{session_id}]]");
            let mut related = match parent_frontmatter.remove("related") {
                Some(YamlValue::Sequence(values)) => values,
                Some(YamlValue::String(value)) if !value.trim().is_empty() => {
                    vec![YamlValue::String(value)]
                }
                _ => Vec::new(),
            };
            if !related
                .iter()
                .filter_map(YamlValue::as_str)
                .any(|existing| existing == related_link)
            {
                related.push(YamlValue::String(related_link));
            }
            parent_frontmatter.insert("related".to_string(), YamlValue::Sequence(related));
            write_markdown(
                &parent_uri,
                Some(&parent_frontmatter),
                &parent_content,
                &context.config,
            )?;
        }
    }

    if !cancel {
        let response = response.expect("validated response");
        let (heading, section) = build_thought_section(
            thought_number,
            total_thoughts,
            needs_more_thoughts,
            branch_id,
            is_revision,
            revises_thought,
            response,
            thoughts,
        );
        append_h2_section(&uri, &heading, &section, &context.config)?;
    }

    let status = if cancel {
        "cancelled"
    } else if next_thought_needed {
        "in_progress"
    } else {
        if conclusion.is_none() {
            let message = "Conclusion required when completing session. Must synthesize findings with [[WikiLinks]].";
            return Ok(match context.output_format {
                OutputFormat::Json => failure("CONCLUSION_REQUIRED", message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }
        if extract_wikilinks(conclusion.expect("validated conclusion")).is_empty() {
            let message = "Conclusion must include [[WikiLinks]] for knowledge graph integration.";
            return Ok(match context.output_format {
                OutputFormat::Json => failure("CONCLUSION_WIKILINK_REQUIRED", message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }
        append_h2_section(
            &uri,
            "Conclusion",
            conclusion.expect("validated conclusion"),
            &context.config,
        )?;
        "completed"
    };

    if status != "in_progress" {
        let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
        let mut frontmatter = frontmatter.unwrap_or_default();
        frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
        frontmatter.insert(
            "thoughtCount".to_string(),
            YamlValue::Number(serde_yaml::Number::from(thought_number + 1)),
        );
        if status == "completed" {
            frontmatter.insert(
                "completed".to_string(),
                YamlValue::String(now_iso8601_roundtrip()),
            );
        } else if status == "cancelled" {
            frontmatter.insert(
                "cancelled".to_string(),
                YamlValue::String(now_iso8601_roundtrip()),
            );
        }
        write_markdown(&uri, Some(&frontmatter), &content, &context.config)?;
    }

    let message = build_completion_message(
        thought_number,
        &session_id,
        cancel,
        next_thought_needed,
        needs_more_thoughts,
        total_thoughts,
    );

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "sessionId": session_id,
            "thoughtNumber": thought_number,
            "totalThoughts": total_thoughts,
            "status": status,
            "nextThoughtNeeded": next_thought_needed,
            "message": message,
        })),
        OutputFormat::Markdown => message,
    })
}

pub fn workflow(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Workflow")
            .unwrap_or_else(|| "ERROR: Help file not found for Workflow".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let workflow_ids = parse_workflow_identifiers(payload.get("workflowId"))?;
    let session_id = payload.get("sessionId").and_then(Value::as_str);
    let response = payload.get("response").and_then(Value::as_str);
    let thoughts = payload.get("thoughts").and_then(Value::as_str);
    let status = payload.get("status").and_then(Value::as_str);
    let conclusion = payload.get("conclusion").and_then(Value::as_str);
    let view = payload
        .get("view")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let append = payload.get("append");
    let submachine_session_id = payload.get("submachineSessionId").and_then(Value::as_str);

    if workflow_ids.is_some() && session_id.is_some() {
        return Err(MaenifoldError::Message(format!(
            "Cannot provide both workflowId and sessionId together: workflowId={}, sessionId={}",
            payload
                .get("workflowId")
                .map(Value::to_string)
                .unwrap_or_default(),
            session_id.unwrap_or_default()
        )));
    }

    AssetManager::initialize_assets(&context.config)?;

    if let Some(workflow_ids) = workflow_ids {
        return start_workflow_session(context, &workflow_ids);
    }

    if let Some(session_id) = session_id {
        if view {
            return view_workflow_session(context, session_id);
        }
        if let Some(append) = append {
            return append_workflow_queue(context, session_id, append);
        }
        if let Some(response) = response {
            return continue_workflow_session(
                context,
                session_id,
                response,
                thoughts,
                status,
                conclusion,
                submachine_session_id,
            );
        }
    }

    Ok(
        "Workflow usage:\n\n  Start session: workflowId\n  Continue session: sessionId + response\n  View queue: sessionId + view=true\n  Append to queue: sessionId + append\n\nTIP: view=true and append both require a sessionId to identify which session to operate on.".to_string(),
    )
}

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
    let abandonment_minutes = std::env::var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(120);
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
    let statement = content
        .lines()
        .find_map(|line| line.strip_prefix("# Assumption: "))
        .unwrap_or("Unknown assumption");
    let status = frontmatter_string(&frontmatter, "status");
    let confidence = frontmatter_string(&frontmatter, "confidence");
    let context_text = frontmatter_string(&frontmatter, "context");

    let mut output = format!(
        "📋 **Assumption**: {statement}\n\n**Status**: {status}\n**Confidence**: {confidence}"
    );
    if !context_text.is_empty() {
        output.push_str(&format!("\n**Context**: {context_text}"));
    }
    output.push_str("\n\n");
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
#[allow(clippy::too_many_arguments)]
fn build_thought_section(
    thought_number: u64,
    total_thoughts: u64,
    needs_more_thoughts: bool,
    branch_id: Option<&str>,
    is_revision: bool,
    revises_thought: Option<u64>,
    response: &str,
    thoughts: Option<&str>,
) -> (String, String) {
    let display_total = if needs_more_thoughts && thought_number >= total_thoughts {
        thought_number + 1
    } else {
        total_thoughts
    };
    let agent_id = std::env::var("AGENT_ID").unwrap_or_else(|_| "agent".to_string());
    let suffix = if let Some(branch_id) = branch_id.filter(|value| !value.is_empty()) {
        format!(" (Branch: {branch_id})")
    } else if is_revision {
        revises_thought
            .map(|value| format!(" (Revises: {value})"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let heading = format!("Thought {thought_number}/{display_total} [{agent_id}]{suffix}");
    let thoughts_section = thoughts
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n\n*Thoughts: {value}*"))
        .unwrap_or_default();
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let content = format!("{response}{thoughts_section}\n\n*{timestamp}*\n");
    (heading, content)
}

fn build_completion_message(
    thought_number: u64,
    session_id: &str,
    cancel: bool,
    next_thought_needed: bool,
    needs_more_thoughts: bool,
    total_thoughts: u64,
) -> String {
    let base_message = if thought_number == 0 {
        format!("Created session: {session_id}")
    } else {
        format!("Added thought {thought_number} to session: {session_id}")
    };

    if cancel {
        return format!("{base_message}\n\n❌ Thinking cancelled");
    }
    if !next_thought_needed {
        return format!("{base_message}\n\n✅ Thinking complete");
    }

    let next_thought = thought_number + 1;
    let next_total = if needs_more_thoughts && thought_number >= total_thoughts.saturating_sub(1) {
        "?".to_string()
    } else {
        total_thoughts.to_string()
    };
    let extend_note = if needs_more_thoughts && thought_number >= total_thoughts.saturating_sub(1) {
        " (extending beyond initial estimate)"
    } else {
        ""
    };
    let checkpoint_note = if thought_number == 0 || thought_number.is_multiple_of(3) {
        "\n\n💡 **CHECK YOUR MEMORY:** `search_memories` for what exists and `build_context` on [[WikiLinks]] | `sync` new findings to add them to the graph"
    } else {
        ""
    };

    format!(
        "{base_message}\n\n💭 Continue with thought {next_thought}/{next_total}{extend_note}{checkpoint_note}"
    )
}

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

fn format_workflow_step(step: &Value) -> Result<String> {
    let pretty = serde_json::to_string_pretty(step)?;
    let mut lines = pretty.lines();
    let Some(first_line) = lines.next() else {
        return Ok(String::new());
    };
    let mut rendered = String::from(first_line);
    for line in lines {
        rendered.push('\n');
        rendered.push_str("    ");
        rendered.push_str(line);
    }
    Ok(rendered)
}

fn start_workflow_session(context: &ToolContext, workflow_ids: &[String]) -> Result<String> {
    let workflow_id = workflow_ids
        .first()
        .ok_or_else(|| MaenifoldError::MissingProperty("workflowId"))?;
    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    let steps = workflow_steps(&workflow_value, workflow_id)?;
    if steps.is_empty() {
        return Ok(format!("ERROR: Workflow '{workflow_id}' has no steps"));
    }

    let session_id = generate_workflow_session_id();
    let first_step = steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&first_step)?;
    let uri = workflow_session_uri(&session_id);

    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "queue".to_string(),
        yaml_sequence_from_strings(workflow_ids),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );

    let file_content = format!(
        "# Workflow Session\n\nStarted: {}\n\n## Step 1/{}\n\n{}\n",
        display_utc_now(),
        steps.len(),
        rendered_step,
    );
    write_markdown(&uri, Some(&frontmatter), &file_content, &context.config)?;

    Ok(format!(
        "Created session: {session_id}\n\nStep 1/{}\n\n{}",
        steps.len(),
        rendered_step
    ))
}

fn view_workflow_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
    }

    let (frontmatter, _, _) = read_markdown(&uri, &context.config)?;
    let frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let current_step = frontmatter_usize(&frontmatter, "currentStep");
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    let mut result = String::new();
    result.push_str(&format!("Queue: [{}]\n", queue.join(", ")));
    result.push_str(&format!(
        "Position: {} (workflow {}/{}, step {}/{})\n",
        current_workflow_id,
        current_workflow_index + 1,
        queue.len(),
        current_step + 1,
        steps.len()
    ));

    if current_step + 1 < steps.len() {
        let next = steps[current_step + 1]
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Unnamed step");
        result.push_str(&format!("Next: {next}\n"));
    } else if current_workflow_index + 1 < queue.len() {
        result.push_str(&format!(
            "Next workflow: {}\n",
            queue[current_workflow_index + 1]
        ));
    } else {
        result.push_str("Next: Queue will complete\n");
    }

    Ok(result)
}

fn append_workflow_queue(
    context: &ToolContext,
    session_id: &str,
    append: &Value,
) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    let mut queue = read_workflow_queue(&frontmatter);
    let to_append = parse_workflow_identifiers(Some(append))?
        .ok_or(MaenifoldError::MissingProperty("append"))?;

    for workflow_id in &to_append {
        let _ = read_workflow_definition(&context.config, workflow_id)?;
    }

    // T-HSM-001.1d: RTM FR-10.5 - preserve serial workflow queue semantics.
    queue.extend(to_append.iter().cloned());
    frontmatter.insert("queue".to_string(), yaml_sequence_from_strings(&queue));
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)?;

    let append_content = format!(
        "Added workflows to queue: {}\n*{}*",
        to_append.join(", "),
        display_utc_now()
    );
    append_h2_section(&uri, "Queue Updated", &append_content, &context.config)?;

    Ok(format!(
        "Added {} workflow(s) to queue. New queue: [{}]",
        to_append.len(),
        queue.join(", ")
    ))
}

fn continue_workflow_session(
    context: &ToolContext,
    session_id: &str,
    response: &str,
    thoughts: Option<&str>,
    status: Option<&str>,
    conclusion: Option<&str>,
    submachine_session_id: Option<&str>,
) -> Result<String> {
    if let Some(message) = validate_workflow_response_concepts(response, thoughts) {
        return Ok(message);
    }

    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;

    // T-HSM-001.1c: RTM FR-10.4 - block advancement while a submachine remains active.
    if frontmatter_string(&frontmatter, "phase") == "waiting" {
        let active_submachine_session_id =
            frontmatter_string(&frontmatter, "activeSubmachineSessionId");
        let active_submachine_type = frontmatter_string(&frontmatter, "activeSubmachineType");

        if !active_submachine_session_id.is_empty() && !active_submachine_type.is_empty() {
            let submachine_uri =
                session_uri_for_type(&active_submachine_type, &active_submachine_session_id);
            if !file_exists(&submachine_uri, &context.config)? {
                clear_submachine_frontmatter(&mut frontmatter);
                write_markdown(&uri, Some(&frontmatter), &existing_content, &context.config)?;
                append_h2_section(
                    &uri,
                    "Submachine Cleared",
                    &format!(
                        "Registered submachine '{}' session not found (deleted or expired). Clearing waiting state and resuming workflow.\n\n*{}*",
                        active_submachine_session_id,
                        display_utc_now()
                    ),
                    &context.config,
                )?;
                let (updated_frontmatter, _, _) = read_markdown(&uri, &context.config)?;
                frontmatter = updated_frontmatter.unwrap_or(frontmatter);
            } else {
                let (submachine_frontmatter, _, _) =
                    read_markdown(&submachine_uri, &context.config)?;
                let submachine_status = submachine_frontmatter
                    .as_ref()
                    .map(|values| frontmatter_string(values, "status"))
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "active".to_string());

                if matches!(
                    submachine_status.as_str(),
                    "completed" | "cancelled" | "abandoned"
                ) {
                    clear_submachine_frontmatter(&mut frontmatter);
                    let (_, content_after_clear, _) = read_markdown(&uri, &context.config)?;
                    write_markdown(
                        &uri,
                        Some(&frontmatter),
                        &content_after_clear,
                        &context.config,
                    )?;
                    append_h2_section(
                        &uri,
                        "Submachine Resumed",
                        &format!(
                            "Submachine {} {}. Resuming workflow.\n\n*{}*",
                            active_submachine_session_id,
                            submachine_status,
                            display_utc_now()
                        ),
                        &context.config,
                    )?;
                    let (updated_frontmatter, _, _) = read_markdown(&uri, &context.config)?;
                    frontmatter = updated_frontmatter.unwrap_or(frontmatter);
                } else {
                    return Ok(format!(
                        "Workflow is waiting on submachine {active_submachine_type} {active_submachine_session_id}. Complete it first, then call Workflow again."
                    ));
                }
            }
        }
    }

    if let Some(submachine_session_id) = submachine_session_id.filter(|value| !value.is_empty()) {
        return register_workflow_submachine(
            context,
            &uri,
            frontmatter,
            response,
            thoughts,
            submachine_session_id,
        );
    }

    let mut current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let mut current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        &uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    current_step += 1;
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    let (_, existing_content, _) = read_markdown(&uri, &context.config)?;

    if status == Some("completed") {
        return handle_completed_workflow_status(
            context,
            &uri,
            frontmatter,
            current_step,
            conclusion,
        );
    }

    if status == Some("cancelled") {
        return handle_cancelled_workflow_status(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_step,
        );
    }

    if current_step >= steps.len() {
        current_workflow_index += 1;
        if current_workflow_index >= queue.len() {
            return handle_queue_exhaustion(context, &uri, frontmatter, current_step, conclusion);
        }

        return transition_to_next_workflow(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_workflow_index,
            &queue,
        );
    }

    advance_to_next_workflow_step(
        context,
        &uri,
        frontmatter,
        &existing_content,
        current_step,
        steps,
    )
}

fn register_workflow_submachine(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    response: &str,
    thoughts: Option<&str>,
    submachine_session_id: &str,
) -> Result<String> {
    let submachine_type = "sequential";
    let submachine_uri = session_uri_for_type(submachine_type, submachine_session_id);
    if !file_exists(&submachine_uri, &context.config)? {
        return Ok(format!(
            "ERROR: Submachine session '{submachine_session_id}' not found under type '{submachine_type}'. Ensure the SequentialThinking session exists before registering."
        ));
    }

    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        workflow_uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    // T-HSM-001.1b: RTM FR-10.1 - register submachine lifecycle state in workflow frontmatter.
    // T-HSM-001.1c: RTM FR-10.4 - do not advance currentStep while waiting on the submachine.
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("waiting".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(submachine_type.to_string()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(submachine_session_id.to_string()),
    );
    let (_, content_before_register, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_before_register,
        &context.config,
    )?;
    append_h2_section(
        workflow_uri,
        "Submachine Registered",
        &format!(
            "Submachine registered: {submachine_type} {submachine_session_id}\n\n*{}*",
            display_utc_now()
        ),
        &context.config,
    )?;

    Ok(format!(
        "Registered submachine {submachine_session_id}. Workflow is waiting. Complete the submachine, then call Workflow with sessionId and response to continue."
    ))
}

fn handle_completed_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match conclusion {
        Some(conclusion) if !conclusion.is_empty() => conclusion,
        _ => {
            return Ok(
                "ERROR: Conclusion required when completing session. Must synthesize findings with [[WikiLinks]]."
                    .to_string(),
            );
        }
    };

    if extract_wikilinks(conclusion).is_empty() {
        return Ok(
            "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration."
                .to_string(),
        );
    }

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );

    let (_, content_after_conclusion, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_after_conclusion,
        &context.config,
    )?;
    Ok("✅ Workflow session completed".to_string())
}

fn handle_cancelled_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_step: usize,
) -> Result<String> {
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("cancelled".to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    Ok("❌ Workflow session cancelled".to_string())
}

fn handle_queue_exhaustion(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match conclusion {
        Some(conclusion) if !conclusion.is_empty() => conclusion,
        _ => {
            return Ok(
                "ERROR: All workflows complete. Call workflow tool again with conclusion parameter to finalize."
                    .to_string(),
            );
        }
    };

    if extract_wikilinks(conclusion).is_empty() {
        return Ok(
            "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration."
                .to_string(),
        );
    }

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    let (_, content_after_conclusion, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_after_conclusion,
        &context.config,
    )?;
    Ok("✅ All workflows in queue completed".to_string())
}

fn transition_to_next_workflow(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_workflow_index: usize,
    queue: &[String],
) -> Result<String> {
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_workflow_index as i64)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;

    let next_workflow_id = queue.get(current_workflow_index).ok_or_else(|| {
        MaenifoldError::Message("Workflow queue transition index out of range".to_string())
    })?;
    let workflow_value = read_workflow_definition(&context.config, next_workflow_id)?;
    let next_steps = workflow_steps(&workflow_value, next_workflow_id)?;
    let next_step = next_steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step 1/{}", next_steps.len()),
        &rendered_step,
        &context.config,
    )?;

    Ok(format!("Step 1/{}\n\n{}", next_steps.len(), rendered_step))
}

fn advance_to_next_workflow_step(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: Frontmatter,
    existing_content: &str,
    current_step: usize,
    steps: &[Value],
) -> Result<String> {
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    let next_step = steps
        .get(current_step)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step {}/{}", current_step + 1, steps.len()),
        &rendered_step,
        &context.config,
    )?;
    Ok(format!(
        "Step {}/{}\n\n{}",
        current_step + 1,
        steps.len(),
        rendered_step
    ))
}

fn validate_workflow_response_concepts(response: &str, thoughts: Option<&str>) -> Option<String> {
    let total_concepts = extract_wikilinks(response).len()
        + thoughts.map(extract_wikilinks).unwrap_or_default().len();
    if total_concepts == 0 {
        return Some(
            "ERROR: Response or thoughts must contain at least one [[WikiLink]] in double brackets.\nExample: 'Implementing [[REST API]] using [[ASP.NET Core]]'\nThis ensures your workflow contributions build the knowledge graph.".to_string(),
        );
    }
    None
}

fn build_workflow_response_content(response: &str, thoughts: Option<&str>) -> String {
    let mut content = String::new();
    content.push_str(response);
    if let Some(thoughts) = thoughts.filter(|value| !value.is_empty()) {
        content.push_str("\n\n");
        content.push_str(&format!("*Thoughts: {thoughts}*"));
    }
    content.push_str("\n\n");
    content.push_str(&format!("*{}*", display_utc_now()));
    content
}

fn parse_workflow_identifiers(value: Option<&Value>) -> Result<Option<Vec<String>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let ids = match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else if trimmed.starts_with('[') {
                serde_json::from_str::<Vec<String>>(trimmed)?
            } else {
                vec![trimmed.to_string()]
            }
        }
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                MaenifoldError::InvalidArgument("Workflow identifiers must be strings".to_string())
            })?,
        _ => {
            return Err(MaenifoldError::InvalidArgument(
                "Workflow identifiers must be a string or string array".to_string(),
            ));
        }
    };

    if ids.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "At least one workflow identifier is required".to_string(),
        ));
    }

    Ok(Some(ids))
}

fn read_workflow_definition(config: &Config, workflow_id: &str) -> Result<Value> {
    let workflow_path = config
        .assets_path
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let workflow_text = fs::read_to_string(&workflow_path)
        .map_err(|_| MaenifoldError::Message(format!("Workflow '{workflow_id}' not found")))?;
    Ok(serde_json::from_str(&workflow_text)?)
}

fn workflow_steps<'a>(workflow_value: &'a Value, workflow_id: &str) -> Result<&'a Vec<Value>> {
    workflow_value
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Workflow '{workflow_id}' is missing 'steps' property"
            ))
        })
}

fn read_workflow_queue(frontmatter: &Frontmatter) -> Vec<String> {
    match frontmatter.get("queue") {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        Some(YamlValue::String(value)) => {
            let trimmed = value.trim();
            if trimmed.starts_with('[') {
                serde_json::from_str::<Vec<String>>(trimmed).unwrap_or_default()
            } else if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn yaml_sequence_from_strings(values: &[String]) -> YamlValue {
    YamlValue::Sequence(
        values
            .iter()
            .cloned()
            .map(YamlValue::String)
            .collect::<Vec<_>>(),
    )
}

fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    frontmatter
        .get(key)
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string()
}

fn frontmatter_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn clear_submachine_frontmatter(frontmatter: &mut Frontmatter) {
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
}

fn workflow_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/workflow/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn session_uri_for_type(session_type: &str, session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/{session_type}/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn display_utc_now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|part| part.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

fn generate_sequential_session_id() -> String {
    let millis = Utc::now().timestamp_millis();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let suffix = 10_000 + (nanos % 90_000);
    format!("session-{millis}-{suffix}")
}

fn generate_workflow_session_id() -> String {
    format!("workflow-{}", Utc::now().timestamp_millis())
}

fn now_iso8601_roundtrip() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    let path = assets_path
        .join("usage")
        .join("tools")
        .join(format!("{}.md", tool_name.to_ascii_lowercase()));
    fs::read_to_string(path).ok()
}
