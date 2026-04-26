#[cfg(test)]
mod tests {
    use super::super::sequential_thinking;
    use crate::config::Config;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::sessions::frontmatter_string;
    use crate::test_util::ConfigResetGuard;
    use crate::thinking::sequential_session_uri;
    use crate::tools::ToolContext;
    use crate::util::json::OutputFormat;
    use serde_json::json;
    use serde_yml::Value as YamlValue;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

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
