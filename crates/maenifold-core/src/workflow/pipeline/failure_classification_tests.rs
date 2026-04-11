use crate::error::MaenifoldError;
use crate::workflow::runner::{WorkflowResult, WorkflowRunStatus};

use super::*;

#[test]
fn failure_phase_pre_execution_is_retryable() {
    assert!(FailurePhase::PreExecution.is_retryable());
}

#[test]
fn failure_phase_post_prompt_is_not_retryable() {
    assert!(!FailurePhase::PostPrompt.is_retryable());
}

#[test]
fn classify_prompt_error_produces_abandoned_with_phase() {
    let outcome = classify_node_result(
        "prompt-node",
        NodeTaskResult::Prompt(Err(MaenifoldError::Message(
            "prompt execution failed".to_string(),
        ))),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, phase, .. } => {
            assert_eq!(reason, "prompt execution failed");
            assert_eq!(phase, FailurePhase::PostPrompt);
        }
        NodeOutcome::Completed { .. } => panic!("prompt errors should abandon the node"),
    }
}

#[test]
fn classify_agent_setup_error_is_pre_execution() {
    let outcome = classify_node_result(
        "prompt-node",
        NodeTaskResult::Prompt(Err(MaenifoldError::AgentSetup(
            "session setup failed".to_string(),
        ))),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, phase, .. } => {
            assert_eq!(reason, "session setup failed");
            assert_eq!(phase, FailurePhase::PreExecution);
        }
        NodeOutcome::Completed { .. } => panic!("agent setup errors should abandon the node"),
    }
}

#[test]
fn classify_workflow_cancelled_is_post_prompt() {
    let outcome = classify_node_result(
        "workflow-node",
        NodeTaskResult::Workflow(Ok(WorkflowResult {
            session_id: "workflow-1".to_string(),
            acp_session_id: Some("acp-workflow-1".to_string()),
            conclusion: None,
            steps_completed: 1,
            status: WorkflowRunStatus::Cancelled,
        })),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, phase, .. } => {
            assert_eq!(reason, "workflow cancelled");
            assert_eq!(phase, FailurePhase::PostPrompt);
        }
        NodeOutcome::Completed { .. } => panic!("cancelled workflows should abandon the node"),
    }
}

#[test]
fn classify_workflow_failed_is_post_prompt() {
    let outcome = classify_node_result(
        "workflow-node",
        NodeTaskResult::Workflow(Ok(WorkflowResult {
            session_id: "workflow-2".to_string(),
            acp_session_id: Some("acp-workflow-2".to_string()),
            conclusion: None,
            steps_completed: 1,
            status: WorkflowRunStatus::Failed("workflow failed".to_string()),
        })),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, phase, .. } => {
            assert_eq!(reason, "workflow failed");
            assert_eq!(phase, FailurePhase::PostPrompt);
        }
        NodeOutcome::Completed { .. } => panic!("failed workflows should abandon the node"),
    }
}

#[test]
fn classify_empty_response_is_post_prompt() {
    let outcome = classify_node_result(
        "prompt-node",
        NodeTaskResult::Prompt(Ok(PromptExecutionResult {
            response: " \n\t ".to_string(),
            acp_session_id: Some("acp-prompt-empty".to_string()),
        })),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, phase, .. } => {
            assert_eq!(reason, "empty response");
            assert_eq!(phase, FailurePhase::PostPrompt);
        }
        NodeOutcome::Completed { .. } => panic!("empty prompt output should abandon the node"),
    }
}
