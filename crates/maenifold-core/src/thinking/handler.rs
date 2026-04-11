use crate::error::Result;
use crate::markdown::{append_h2_section, read_markdown};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use crate::workflow::{
    HostSessionType, ResumeSubmachineResult, maybe_resume_submachine, parse_submachine_type,
    register_host_submachine,
};
use serde_json::{Value, json};

use super::formatting::{build_completion_message, build_thought_section};
use super::render_text_success;
use super::session::{
    finalize_session, initialize_session, load_help_doc, sequential_session_uri,
    validate_session_state,
};
use super::validation::{SequentialThinkingInput, validate_completion, validate_payload};

// T-12.0: RTM NFR-1 - preserve SequentialThinking behavior while delegating cohesive helper groups.
#[allow(clippy::too_many_lines)]
pub fn sequential_thinking(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "SequentialThinking")
            .unwrap_or_else(|| "ERROR: Help file not found for SequentialThinking".to_string());
        return Ok(render_text_success(context, help));
    }

    let input = SequentialThinkingInput::from_payload(payload);
    if let Some(output) = validate_payload(context, &input)? {
        return Ok(output);
    }

    let uri = sequential_session_uri(&input.session_id);
    let (session_exists, early_output) = validate_session_state(context, &input, &uri)?;
    if let Some(output) = early_output {
        return Ok(output);
    }

    if !session_exists {
        initialize_session(context, &input, &uri)?;
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();
    match maybe_resume_submachine(
        context,
        HostSessionType::Sequential,
        &uri,
        frontmatter,
        &existing_content,
    )? {
        ResumeSubmachineResult::Ready(updated_frontmatter) => frontmatter = updated_frontmatter,
        ResumeSubmachineResult::Waiting(message) => return Ok(message),
    }

    if let Some(submachine_session_id) = input
        .submachine_session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let response = input.response.as_deref().unwrap_or("");
        let (heading, section) = build_thought_section(
            input.thought_number,
            input.total_thoughts,
            input.needs_more_thoughts,
            input.branch_id.as_deref(),
            input.is_revision,
            input.revises_thought,
            response,
            input.thoughts.as_deref(),
        );
        append_h2_section(&uri, &heading, &section, &context.config)?;
        let submachine_type = parse_submachine_type(input.submachine_type.as_deref())?;
        return register_host_submachine(
            context,
            HostSessionType::Sequential,
            &input.session_id,
            &uri,
            &mut frontmatter,
            submachine_session_id,
            submachine_type,
        );
    }

    if !input.cancel {
        // PARITY-T07: RTM NFR-1 — response may be None when thoughts carried the WikiLinks.
        let response = input.response.as_deref().unwrap_or("");
        let (heading, section) = build_thought_section(
            input.thought_number,
            input.total_thoughts,
            input.needs_more_thoughts,
            input.branch_id.as_deref(),
            input.is_revision,
            input.revises_thought,
            response,
            input.thoughts.as_deref(),
        );
        append_h2_section(&uri, &heading, &section, &context.config)?;
    }

    let status = if input.cancel {
        "cancelled"
    } else if input.next_thought_needed {
        "in_progress"
    } else {
        if let Some(output) = validate_completion(context, &input)? {
            return Ok(output);
        }
        append_h2_section(
            &uri,
            "Conclusion",
            input.conclusion.as_deref().expect("validated conclusion"),
            &context.config,
        )?;
        "completed"
    };

    if status != "in_progress" {
        finalize_session(context, &uri, status, input.thought_number)?;
    }

    let message = build_completion_message(
        input.thought_number,
        &input.session_id,
        input.cancel,
        input.next_thought_needed,
        input.needs_more_thoughts,
        input.total_thoughts,
    );

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "sessionId": input.session_id,
            "thoughtNumber": input.thought_number,
            "totalThoughts": input.total_thoughts,
            "status": status,
            "nextThoughtNeeded": input.next_thought_needed,
            "message": message,
        })),
        OutputFormat::Markdown => message,
    })
}

#[cfg(test)]
mod tests {
    use super::sequential_thinking;
    use crate::config::Config;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::sessions::frontmatter_string;
    use crate::thinking::sequential_session_uri;
    use crate::tools::ToolContext;
    use crate::util::json::OutputFormat;
    use serde_json::json;
    use serde_yaml::Value as YamlValue;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        (
            tmp,
            ToolContext::new(config, OutputFormat::Markdown),
            ConfigResetGuard,
        )
    }

    fn seed_session(context: &ToolContext, session_id: &str) {
        let mut frontmatter: Frontmatter = BTreeMap::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Sequential Thinking Session {session_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(session_id.to_string()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String("sequential".to_string()),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
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
        frontmatter.insert(
            "activeSubmachineTarget".to_string(),
            YamlValue::String(String::new()),
        );
        write_markdown(
            &sequential_session_uri(session_id),
            Some(&frontmatter),
            "# Sequential Thinking Session\n\n",
            &context.config,
        )
        .expect("seed session");
    }

    #[test]
    fn sequential_submachine_waits_and_resumes_before_advancing() {
        let (_tmp, context, _guard) = test_context();
        let parent_session_id = "session-1735689600000-10001";
        let child_session_id = "session-1735689600001-10002";

        seed_session(&context, child_session_id);
        seed_session(&context, parent_session_id);

        let waiting = sequential_thinking(
            &context,
            &json!({
                "sessionId": parent_session_id,
                "thoughtNumber": 1,
                "totalThoughts": 2,
                "response": "[[parent]] waiting",
                "submachineSessionId": child_session_id,
            }),
        )
        .expect("register child");
        assert!(waiting.contains("waiting"));

        let (frontmatter, _, _) =
            read_markdown(&sequential_session_uri(parent_session_id), &context.config)
                .expect("read parent");
        let frontmatter = frontmatter.expect("frontmatter");
        assert_eq!(frontmatter_string(&frontmatter, "phase"), "waiting");
        assert_eq!(
            frontmatter_string(&frontmatter, "activeSubmachineType"),
            "sequential"
        );

        let blocked = sequential_thinking(
            &context,
            &json!({
                "sessionId": parent_session_id,
                "thoughtNumber": 2,
                "totalThoughts": 2,
                "response": "[[parent]] blocked",
                "conclusion": "[[parent]] done",
            }),
        )
        .expect("waiting message");
        assert!(blocked.contains("waiting on submachine"));

        sequential_thinking(
            &context,
            &json!({
                "sessionId": child_session_id,
                "thoughtNumber": 1,
                "totalThoughts": 2,
                "response": "[[child]] done",
                "nextThoughtNeeded": false,
                "conclusion": "[[child]] complete",
            }),
        )
        .expect("complete child");

        sequential_thinking(
            &context,
            &json!({
                "sessionId": parent_session_id,
                "thoughtNumber": 2,
                "totalThoughts": 2,
                "response": "[[parent]] resumed",
                "nextThoughtNeeded": false,
                "conclusion": "[[parent]] complete",
            }),
        )
        .expect("resume parent");

        let (frontmatter, _, _) =
            read_markdown(&sequential_session_uri(parent_session_id), &context.config)
                .expect("read parent after resume");
        let frontmatter = frontmatter.expect("frontmatter after resume");
        assert_eq!(frontmatter_string(&frontmatter, "phase"), "running");
        assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
        assert_eq!(frontmatter_string(&frontmatter, "activeSubmachineType"), "");
        assert_eq!(
            frontmatter_string(&frontmatter, "activeSubmachineSessionId"),
            ""
        );
        assert_eq!(
            frontmatter_string(&frontmatter, "activeSubmachineTarget"),
            ""
        );
    }

    #[test]
    fn completed_sequential_session_rejects_additional_activity() {
        let (_tmp, context, _guard) = test_context();
        let session_id = "session-1735689600000-10001";
        seed_session(&context, session_id);

        sequential_thinking(
            &context,
            &json!({
                "sessionId": session_id,
                "thoughtNumber": 1,
                "totalThoughts": 1,
                "response": "[[child]] done",
                "nextThoughtNeeded": false,
                "conclusion": "[[child]] complete",
            }),
        )
        .expect("complete session");

        let result = sequential_thinking(
            &context,
            &json!({
                "sessionId": session_id,
                "thoughtNumber": 2,
                "totalThoughts": 2,
                "response": "[[child]] mutate",
                "nextThoughtNeeded": true,
            }),
        )
        .expect("closed-session rejection");

        assert!(result.contains("already completed"));
    }
}
