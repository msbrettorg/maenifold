use super::executor::{execute_with_retry_policy, with_retry_attempts};
use super::executor_nodes::NodeOutcome;
use super::executor_run::FailurePhase;
use super::types::RetryPolicy;

#[tokio::test]
async fn transient_failure_recovers_within_retry_budget() {
    let mut attempts = 0u32;
    let policy = RetryPolicy {
        max_attempts: 3,
        backoff_min_ms: 1,
        backoff_max_ms: 5,
        jitter: false,
    };

    let (outcome, retry_attempts) = execute_with_retry_policy("node-1", policy, || {
        attempts += 1;
        async move {
            if attempts < 3 {
                Err(super::executor::RetryExecutionError {
                    kind: "timeout",
                    reason: "timeout after 1s".to_string(),
                })
            } else {
                Ok(NodeOutcome::Completed {
                    conclusion: "done".to_string(),
                    acp_session_id: None,
                    thinking_session_id: None,
                    tool_provenance: None,
                    retry_attempts: 0,
                })
            }
        }
    })
    .await;

    let outcome = with_retry_attempts(outcome, retry_attempts);
    match outcome {
        NodeOutcome::Completed {
            conclusion,
            retry_attempts,
            ..
        } => {
            assert_eq!(conclusion, "done");
            assert_eq!(retry_attempts, 2);
        }
        NodeOutcome::Abandoned { reason, .. } => {
            panic!("expected completion after transient retries, got: {reason}")
        }
    }
}

#[tokio::test]
async fn persistent_transient_failure_exhausts_budget_and_abandons() {
    let policy = RetryPolicy {
        max_attempts: 2,
        backoff_min_ms: 1,
        backoff_max_ms: 5,
        jitter: false,
    };

    let (outcome, retry_attempts) = execute_with_retry_policy("node-2", policy, || async {
        Err(super::executor::RetryExecutionError {
            kind: "timeout",
            reason: "timeout after 5s".to_string(),
        })
    })
    .await;
    let outcome = with_retry_attempts(outcome, retry_attempts);

    match outcome {
        NodeOutcome::Abandoned {
            reason,
            phase,
            retry_attempts,
            ..
        } => {
            assert_eq!(phase, FailurePhase::PostPrompt);
            assert_eq!(retry_attempts, 2);
            assert!(
                reason.contains("abandoned after 2 transient retries: timeout after 5s"),
                "unexpected failure_reason: {reason}"
            );
        }
        NodeOutcome::Completed { .. } => {
            panic!("expected abandoned outcome when retries are exhausted")
        }
    }
}
