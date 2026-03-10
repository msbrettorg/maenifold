use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, failure, success};
use crate::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::sessions::{
    date_parts_from_session_id, extract_session_timestamp_millis, frontmatter_string,
    now_iso8601_roundtrip, workflow_session_uri,
};
use crate::tools::ToolContext;
use chrono::Utc;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// T-11.8: RTM FR-11 - extract SequentialThinking handling into a dedicated module.
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

    // T-11.8: RTM FR-11 - preserve sequential session ID validation during extraction.
    if session_id_provided && !is_valid_session_id(&session_id) {
        let message = format!("Invalid sessionId format: {session_id}");
        return Ok(match context.output_format {
            OutputFormat::Json => failure("INVALID_SESSION_ID", &message),
            OutputFormat::Markdown => format!("ERROR: {message}"),
        });
    }

    if !cancel && thought_number > 0 && payload.get("sessionId").is_none() {
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

        let (parent_frontmatter, _, _) = read_markdown(&parent_uri, &context.config)?;
        let parent_frontmatter = parent_frontmatter.unwrap_or_default();
        let parent_status = frontmatter_string(&parent_frontmatter, "status");
        let normalized_parent_status = parent_status.trim().to_ascii_lowercase();
        if ["completed", "cancelled", "abandoned"]
            .iter()
            .any(|closed| *closed == normalized_parent_status)
        {
            let message =
                format!("Parent workflow {parent_workflow_id} is {normalized_parent_status}");
            return Ok(match context.output_format {
                OutputFormat::Json => failure("PARENT_WORKFLOW_CLOSED", &message),
                OutputFormat::Markdown => format!("ERROR: {message}"),
            });
        }
    }

    let uri = sequential_session_uri(&session_id);
    let session_exists = file_exists(&uri, &context.config)?;

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

fn sequential_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

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

fn is_valid_session_id(session_id: &str) -> bool {
    extract_session_timestamp_millis(session_id).is_some()
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
