use super::*;

#[test]
fn classify_prompt_result_abandons_whitespace_only_output() {
    let outcome = classify_prompt_result(
        "prompt-node",
        Ok(PromptExecutionResult {
            response: "  \n\t  ".to_string(),
            acp_session_id: Some("acp-prompt-1".to_string()),
            input_tokens: None,
            output_tokens: None,
            model: None,
        }),
        None,
    );

    match outcome {
        NodeOutcome::Abandoned { reason, .. } => assert_eq!(reason, "empty response"),
        NodeOutcome::Completed { .. } => {
            panic!("whitespace-only prompt output should be abandoned")
        }
    }
}

#[test]
fn classify_prompt_result_completes_non_empty_output() {
    let outcome = classify_prompt_result(
        "prompt-node",
        Ok(PromptExecutionResult {
            response: "  kept  ".to_string(),
            acp_session_id: Some("acp-prompt-2".to_string()),
            input_tokens: None,
            output_tokens: None,
            model: None,
        }),
        None,
    );

    match outcome {
        NodeOutcome::Completed {
            conclusion,
            acp_session_id,
            ..
        } => {
            assert_eq!(conclusion, "  kept  ");
            assert_eq!(acp_session_id.as_deref(), Some("acp-prompt-2"));
        }
        NodeOutcome::Abandoned { reason, .. } => {
            panic!("non-empty prompt output should complete, got: {reason}")
        }
    }
}

#[test]
fn classify_tool_result_records_structured_provenance() {
    let outcome = classify_tool_result(
        "tool-node",
        Ok(ToolExecutionResult {
            response: "{\"success\":true}".to_string(),
            provenance: ToolProvenance {
                tool_name: "example.echo".to_string(),
                resolved_args: Some(serde_json::json!({ "value": 1 })),
                response_summary: "{\"success\":true}".to_string(),
            },
        }),
    );

    match outcome {
        NodeOutcome::Completed {
            conclusion,
            tool_provenance,
            ..
        } => {
            assert_eq!(conclusion, "{\"success\":true}");
            let provenance = tool_provenance.expect("tool provenance should be captured");
            assert_eq!(provenance.tool_name, "example.echo");
            assert_eq!(
                provenance.resolved_args,
                Some(serde_json::json!({ "value": 1 }))
            );
            assert_eq!(provenance.response_summary, "{\"success\":true}");
        }
        NodeOutcome::Abandoned { reason, .. } => {
            panic!("successful tool output should complete, got: {reason}")
        }
    }
}
