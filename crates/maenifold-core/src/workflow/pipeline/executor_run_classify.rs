use super::*;

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
                NodeOutcome::Abandoned {
                    reason: msg.clone(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: workflow.acp_session_id.clone(),
                    thinking_session_id: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
            WorkflowRunStatus::Cancelled => {
                eprintln!("[pipeline] node {node_id} workflow was cancelled, marking abandoned");
                NodeOutcome::Abandoned {
                    reason: "workflow cancelled".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: workflow.acp_session_id,
                    thinking_session_id: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
        },
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
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
                NodeOutcome::Abandoned {
                    reason,
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    input_tokens: result.input_tokens,
                    output_tokens: result.output_tokens,
                    model: result.model,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
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
                NodeOutcome::Abandoned {
                    reason: "empty response".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    input_tokens: result.input_tokens,
                    output_tokens: result.output_tokens,
                    model: result.model,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
        }
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
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
                        NodeOutcome::Abandoned {
                            reason,
                            phase: FailurePhase::PostPrompt,
                            transient_error_kind: None,
                            acp_session_id: acp_session_id.clone(),
                            thinking_session_id: Some(session_id.clone()),
                            input_tokens: None,
                            output_tokens: None,
                            model: None,
                            retry_attempts: 0,
                            is_outer_timeout: false,
                        }
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
                .unwrap_or(NodeOutcome::Abandoned {
                    reason: "missing conclusion".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id,
                    thinking_session_id: Some(session_id),
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                })
        }
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id,
                input_tokens: None,
                output_tokens: None,
                model: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
    }
}

pub(super) fn classify_tool_result(
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
        Ok(_) => NodeOutcome::Abandoned {
            reason: "empty response".to_string(),
            phase: FailurePhase::PostPrompt,
            transient_error_kind: None,
            acp_session_id: None,
            thinking_session_id: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);

            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id: None,
                thinking_session_id: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
    }
}
