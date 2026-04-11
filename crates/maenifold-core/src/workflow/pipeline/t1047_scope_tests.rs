use super::executor_nodes::{NodeOutcome, NodeTaskResult, classify_node_result};

#[test]
fn classify_node_result_rejects_empty_and_whitespace_only_prompt_output() {
    for response in ["", " \n\t "] {
        let outcome = classify_node_result(
            "prompt-node",
            NodeTaskResult::Prompt(Ok(super::executor_run::PromptExecutionResult {
                response: response.into(),
                acp_session_id: Some("acp-prompt-scope".to_string()),
            })),
        );

        match outcome {
            NodeOutcome::Abandoned { reason, .. } => assert_eq!(reason, "empty response"),
            NodeOutcome::Completed { .. } => {
                panic!("empty or whitespace-only prompt output must be abandoned")
            }
        }
    }
}
