use super::*;

pub(in super::super) fn classify_tool_result(
    node_id: &str,
    result: Result<ToolExecutionResult>,
) -> NodeOutcome {
    match result {
        Ok(result) if !result.response.trim().is_empty() => NodeOutcome::Completed {
            conclusion: result.response,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: Some(result.provenance),
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
        },
        Ok(_) => abandoned(
            "empty response".to_string(),
            FailurePhase::PostPrompt,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        Err(MaenifoldError::AgentSetup(message)) => abandoned(
            log_node_execution_error_str(node_id, &message),
            FailurePhase::PreExecution,
            Some("agent-setup"),
            None,
            None,
            None,
            None,
            None,
        ),
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            abandoned(
                reason,
                phase,
                transient_error_kind,
                None,
                None,
                None,
                None,
                None,
            )
        }
    }
}
