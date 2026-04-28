use super::*;

#[path = "executor_run_classify_tool.rs"]
mod tool;
pub(super) use tool::classify_tool_result;

fn abandoned(
    reason: String,
    phase: FailurePhase,
    transient_error_kind: Option<&'static str>,
    acp_session_id: Option<String>,
    thinking_session_id: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    model: Option<String>,
) -> NodeOutcome {
    NodeOutcome::Abandoned {
        reason,
        phase,
        transient_error_kind,
        acp_session_id,
        thinking_session_id,
        input_tokens,
        output_tokens,
        model,
        retry_attempts: 0,
        is_outer_timeout: false,
    }
}

pub(super) fn classify_workflow_result(
    node_id: &str,
    result: Result<runner::WorkflowResult>,
    acp_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(workflow) => match workflow.status {
            WorkflowRunStatus::Completed => NodeOutcome::Completed {
                conclusion: truncate_chars(
                    &workflow.conclusion.unwrap_or_default(),
                    MAX_CONCLUSION_CHARS,
                ),
                acp_session_id: workflow.acp_session_id,
                thinking_session_id: None,
                tool_provenance: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                retry_attempts: 0,
            },
            WorkflowRunStatus::Failed(ref msg) => {
                eprintln!("[pipeline] node {node_id} workflow failed: {msg}");
                abandoned(
                    msg.clone(),
                    FailurePhase::PostPrompt,
                    None,
                    workflow.acp_session_id.clone(),
                    None,
                    None,
                    None,
                    None,
                )
            }
            WorkflowRunStatus::Cancelled => {
                eprintln!("[pipeline] node {node_id} workflow was cancelled, marking abandoned");
                abandoned(
                    "workflow cancelled".to_string(),
                    FailurePhase::PostPrompt,
                    None,
                    workflow.acp_session_id,
                    None,
                    None,
                    None,
                    None,
                )
            }
        },
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
                acp_session_id,
                None,
                None,
                None,
                None,
            )
        }
    }
}

pub(super) fn classify_prompt_result(
    node_id: &str,
    result: Result<PromptExecutionResult>,
    acp_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(result) => {
            if let Some(reason) = decode_prompt_failure(&result.response) {
                eprintln!("[pipeline] node {node_id} failed: {reason}");
                abandoned(
                    reason,
                    FailurePhase::PostPrompt,
                    None,
                    result.acp_session_id,
                    None,
                    result.input_tokens,
                    result.output_tokens,
                    result.model,
                )
            } else if !result.response.trim().is_empty() {
                NodeOutcome::Completed {
                    conclusion: result.response,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    tool_provenance: None,
                    input_tokens: result.input_tokens,
                    output_tokens: result.output_tokens,
                    model: result.model,
                    retry_attempts: 0,
                }
            } else {
                abandoned(
                    "empty response".to_string(),
                    FailurePhase::PostPrompt,
                    None,
                    result.acp_session_id,
                    None,
                    result.input_tokens,
                    result.output_tokens,
                    result.model,
                )
            }
        }
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
                acp_session_id,
                None,
                None,
                None,
                None,
            )
        }
    }
}

pub(super) fn classify_think_result(
    node_id: &str,
    result: Result<runner::ThinkResult>,
    acp_session_id: Option<String>,
    thinking_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(think) => {
            let runner::ThinkResult {
                session_id,
                acp_session_id,
                conclusion,
                ..
            } = think;
            conclusion
                .map(|conclusion| conclusion.trim().to_string())
                .filter(|conclusion| !conclusion.is_empty())
                .map(|conclusion| {
                    if let Some(reason) = decode_think_failure(&conclusion) {
                        abandoned(
                            reason,
                            FailurePhase::PostPrompt,
                            None,
                            acp_session_id.clone(),
                            Some(session_id.clone()),
                            None,
                            None,
                            None,
                        )
                    } else {
                        NodeOutcome::Completed {
                            conclusion,
                            acp_session_id: acp_session_id.clone(),
                            thinking_session_id: Some(session_id.clone()),
                            tool_provenance: None,
                            input_tokens: None,
                            output_tokens: None,
                            model: None,
                            retry_attempts: 0,
                        }
                    }
                })
                .unwrap_or_else(|| {
                    abandoned(
                        "missing conclusion".to_string(),
                        FailurePhase::PostPrompt,
                        None,
                        acp_session_id,
                        Some(session_id),
                        None,
                        None,
                        None,
                    )
                })
        }
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
                acp_session_id,
                thinking_session_id,
                None,
                None,
                None,
            )
        }
    }
}
